use crate::{Result, storage};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        class: u32,
        buffer: *mut std::ffi::c_void,
        length: u32,
        returned: *mut u32,
    ) -> i32;
    fn NtSetSystemInformation(class: u32, buffer: *mut std::ffi::c_void, length: u32) -> i32;
    fn RtlAdjustPrivilege(privilege: u32, enable: u8, current_thread: u8, previous: *mut u8)
    -> i32;
}
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub total: u64,
    pub available: u64,
    pub standby: u64,
}
impl Snapshot {
    pub fn json(self) -> Value {
        json!({"total":self.total,"available":self.available,"standby":self.standby,"availablePercent":self.available as f64/self.total as f64*100.0})
    }
}
pub fn snapshot() -> Result<Snapshot> {
    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut mem) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut data = [0u64; 22];
    let mut returned = 0;
    let status =
        unsafe { NtQuerySystemInformation(80, data.as_mut_ptr().cast(), 176, &mut returned) };
    if status != 0 {
        return Err(format!(
            "NtQuerySystemInformation failed (NTSTATUS 0x{:x})",
            status as u32
        ));
    }
    let standby = data[5..13].iter().sum::<u64>() * 4096;
    Ok(Snapshot {
        total: mem.ullTotalPhys,
        available: mem.ullAvailPhys,
        standby,
    })
}
pub fn purge_standby() -> Result<()> {
    let mut previous = 0;
    let status = unsafe { RtlAdjustPrivilege(13, 1, 0, &mut previous) };
    if status != 0 {
        return Err(format!(
            "RtlAdjustPrivilege failed (NTSTATUS 0x{:x})",
            status as u32
        ));
    }
    let mut command = 4u32;
    let status = unsafe { NtSetSystemInformation(80, (&mut command as *mut u32).cast(), 4) };
    if status != 0 {
        return Err(format!(
            "NtSetSystemInformation failed (NTSTATUS 0x{:x})",
            status as u32
        ));
    }
    Ok(())
}
pub struct Cleaner {
    pub available_trigger: f64,
    pub standby_trigger: f64,
    pub rearm_available: f64,
    pub cooldown_ms: u64,
    pub armed: bool,
    pub last_cleanup: u64,
    pub error: Option<String>,
}
impl Cleaner {
    pub fn new(settings: &Value, total: u64) -> Self {
        let threshold = |percent: &str, min: &str, max: &str, p: f64, lo: f64, hi: f64| {
            (total as f64 * settings[percent].as_f64().unwrap_or(p) / 100.0).clamp(
                settings[min].as_f64().unwrap_or(lo) * 1048576.0,
                settings[max].as_f64().unwrap_or(hi) * 1048576.0,
            )
        };
        Self {
            available_trigger: threshold(
                "availablePercent",
                "availableMinMiB",
                "availableMaxMiB",
                10.0,
                768.0,
                2048.0,
            ),
            standby_trigger: threshold(
                "standbyPercent",
                "standbyMinMiB",
                "standbyMaxMiB",
                5.0,
                256.0,
                1024.0,
            ),
            rearm_available: threshold(
                "rearmPercent",
                "rearmMinMiB",
                "rearmMaxMiB",
                15.0,
                1536.0,
                4096.0,
            ),
            cooldown_ms: settings["cooldownMs"].as_u64().unwrap_or(600000),
            armed: true,
            last_cleanup: 0,
            error: None,
        }
    }
    // Pure decision logic permits testing without purging the user's memory.
    pub fn should_purge(&mut self, before: Snapshot, now: u64, game_active: bool) -> bool {
        if !game_active {
            return false;
        }
        let cooldown = now.saturating_sub(self.last_cleanup) >= self.cooldown_ms;
        if !self.armed {
            if before.available as f64 >= self.rearm_available && cooldown {
                self.armed = true;
            }
            return false;
        }
        (before.available as f64) < self.available_trigger
            && (before.standby as f64) > self.standby_trigger
            && cooldown
    }
    pub fn purge(&mut self) -> bool {
        self.armed = false;
        match snapshot().and_then(|_| purge_standby()) {
            Ok(()) => {
                self.last_cleanup = crate::now_ms();
                self.error = None;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub fn status(&self, current: Snapshot) -> Value {
        json!({"armed":self.armed,"lastCleanup":if self.last_cleanup==0{Value::Null}else{json!(self.last_cleanup)},"error":self.error,
            "current":current.json(),"thresholds":{"availableTrigger":self.available_trigger,"standbyTrigger":self.standby_trigger,"rearmAvailable":self.rearm_available,"cooldownMs":self.cooldown_ms}})
    }
}
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let arg = |name: &str| {
        args.iter()
            .find_map(|s| s.strip_prefix(&format!("--{name}=")))
            .map(str::to_owned)
    };
    let status = arg("status-path").ok_or("메모리 helper 상태 경로가 없습니다")?;
    let control = arg("control-path").ok_or("메모리 helper 제어 경로가 없습니다")?;
    let parent = arg("parent-pid")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|p| *p > 0)
        .ok_or("메모리 helper 부모 PID가 없습니다")?;
    let status = Path::new(&status);
    let control = Path::new(&control);
    let _lock = storage::HelperLock::acquire(status, parent, control)?;
    let config = storage::read_json(&root.join("config.json"))?.ok_or("config.json missing")?;
    let initial = snapshot()?;
    let mut cleaner = Cleaner::new(&config["memoryCleaner"], initial.total);
    let interval = config["memoryCleaner"]["pollIntervalMs"]
        .as_u64()
        .unwrap_or(1000)
        .max(10);
    let write =
        |cleaner: &Cleaner, game: bool, running: bool, failure: Option<&str>| -> Result<()> {
            let mut data = cleaner.status(snapshot()?);
            data["running"] = json!(running);
            data["gameActive"] = json!(game);
            data["helperPid"] = json!(std::process::id());
            data["parentPid"] = json!(parent);
            data["updatedAt"] = json!(crate::now_ms());
            if !running {
                data["stoppedAt"] = json!(crate::now_ms());
            }
            if let Some(e) = failure {
                data["error"] = json!({"message":e});
            }
            storage::write_json(status, &data)
        };
    let outcome = (|| {
        if arg("purge-on-start").as_deref() == Some("true") {
            cleaner.purge();
        }
        write(&cleaner, false, true, None)?;
        let mut game_watch = crate::process::GameWatch::default();
        while crate::process::alive(parent) {
            if storage::read_json(control)?.is_some_and(|v| v["command"] == "stop") {
                break;
            }
            let game = game_watch.active(&config)?;
            if game {
                match snapshot() {
                    Ok(before) => {
                        if cleaner.should_purge(before, crate::now_ms(), true) {
                            cleaner.purge();
                        }
                    }
                    Err(e) => {
                        cleaner.armed = false;
                        cleaner.error = Some(e);
                    }
                }
            }
            // Temporary file sharing errors do not terminate a healthy worker.
            if let Err(e) = write(&cleaner, game, true, None) {
                eprintln!("메모리 상태 기록 실패: {e}");
            }
            std::thread::sleep(Duration::from_millis(interval));
        }
        Ok(())
    })();
    let final_status = write(
        &cleaner,
        false,
        false,
        outcome.as_ref().err().map(String::as_str),
    );
    outcome.and(final_status)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_recovery_and_cooldown_match_original() {
        let mut c = Cleaner::new(&json!({}), 16 * 1024 * 1024 * 1024);
        let low = Snapshot {
            total: 16 * 1024 * 1024 * 1024,
            available: 500 * 1048576,
            standby: 2 * 1024 * 1024 * 1024,
        };
        assert!(!c.should_purge(low, 1000000, false));
        assert!(c.should_purge(low, 1000000, true));
        c.armed = false;
        c.last_cleanup = 1000000;
        assert!(!c.should_purge(low, 2000000, true));
        assert!(!c.armed);
        let high = Snapshot {
            available: 4 * 1024 * 1024 * 1024,
            ..low
        };
        assert!(!c.should_purge(high, 1100000, true));
        assert!(!c.armed);
        assert!(!c.should_purge(high, 2000000, true));
        assert!(c.armed);
        assert!(c.should_purge(low, 2000001, true));
    }
    #[test]
    fn native_memory_query_is_read_only() {
        let s = snapshot().unwrap();
        assert!(s.total > 0);
        assert!(s.available <= s.total);
    }
}
