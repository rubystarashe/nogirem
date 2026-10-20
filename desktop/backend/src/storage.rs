use crate::Result;
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn read_json(path: &Path) -> Result<Option<Value>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}
pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    let parent = path.parent().ok_or("JSON path has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string(value)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        let delays = [15, 30, 60, 120, 240];
        for attempt in 0..=delays.len() {
            match fs::rename(&temporary, path) {
                Ok(()) => return Ok(()),
                Err(e)
                    if attempt < delays.len()
                        && matches!(e.raw_os_error(), Some(5 | 32 | 33 | 80 | 145 | 183)) =>
                {
                    std::thread::sleep(Duration::from_millis(delays[attempt]))
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        unreachable!()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
// A lock owns its open file until drop; another worker never removes an active
// owner's lock. Existing JS lock records remain readable during migration.
pub struct HelperLock {
    path: PathBuf,
    pid: u32,
    pid_started_at: i64,
    _file: fs::File,
}
fn process_identity_alive(record: &Value) -> bool {
    let Some(pid) = record["pid"].as_u64().and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0) else { return false };
    if !crate::process::alive(pid) { return false; }
    record["pidStartedAt"].as_i64().map(|started_at|crate::process::start_ms(pid).ok()==Some(started_at)).unwrap_or(true)
}
fn lock_owned_by(record: &Value, pid: u32, pid_started_at: i64) -> bool {
    record["pid"].as_u64()==Some(pid as u64) && record["pidStartedAt"].as_i64()==Some(pid_started_at)
}
impl HelperLock {
    pub fn acquire(status: &Path, owner: u32, control: &Path) -> Result<Self> {
        let path = PathBuf::from(format!("{}.lock", status.display()));
        fs::create_dir_all(path.parent().ok_or("Lock has no parent")?)
            .map_err(|e| e.to_string())?;
        let current_pid=std::process::id();
        let current_started_at=crate::process::start_ms(current_pid)?;
        for _ in 0..3 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let record = serde_json::json!({"pid":current_pid,"pidStartedAt":current_started_at,"ownerPid":owner,"startedAt":crate::now_ms()});
                    file.write_all(record.to_string().as_bytes())
                        .map_err(|e| e.to_string())?;
                    file.sync_all().map_err(|e| e.to_string())?;
                    return Ok(Self { path, pid:current_pid, pid_started_at:current_started_at, _file:file });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let previous = read_json(&path).ok().flatten().unwrap_or(Value::Null);
                    let pid = previous["pid"].as_u64().unwrap_or(0) as u32;
                    // Do not unlink a lock being initialized by another process.
                    if previous.is_null() {
                        let age = fs::metadata(&path)
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| t.elapsed().ok())
                            .unwrap_or_default();
                        if age < Duration::from_secs(15) {
                            return Err("Helper lock is being initialized".into());
                        }
                    }
                    if process_identity_alive(&previous) {
                        let old_owner = previous["ownerPid"].as_u64().unwrap_or(0) as u32;
                        if old_owner>0 && owner!=old_owner && !crate::process::alive(old_owner) {
                            write_json(
                                control,
                                &serde_json::json!({"command":"stop","requestedAt":crate::now_ms(),"reason":"orphan-recovery"}),
                            )?;
                            let deadline = std::time::Instant::now() + Duration::from_secs(5);
                            while crate::process::alive(pid) && std::time::Instant::now() < deadline
                            {
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        }
                        if crate::process::alive(pid) {
                            return Err(format!("Helper already running: PID {pid}"));
                        }
                    }
                    match fs::remove_file(&path) {
                        Ok(())=>{},
                        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
                        Err(e)=>return Err(e.to_string()),
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Could not acquire helper lock".into())
    }
}
impl Drop for HelperLock {
    fn drop(&mut self) {
        if read_json(&self.path).ok().flatten().is_some_and(|record|lock_owned_by(&record,self.pid,self.pid_started_at)) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replace_json_without_partial_reads() {
        let dir = std::env::temp_dir().join(format!(
            "nogirem-native-atomic-{}-{}",
            std::process::id(),
            crate::now_ms()
        ));
        let path = dir.join("state.json");
        for n in 0..20 {
            let v = serde_json::json!({"value":n,"text":"한글"});
            write_json(&path, &v).unwrap();
            assert_eq!(read_json(&path).unwrap(), Some(v));
        }
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }
    #[test]
    fn live_lock_is_not_stolen_when_status_is_missing_and_record_is_old() {
        let dir=std::env::temp_dir().join(format!("nogirem-native-live-lock-{}-{}",std::process::id(),crate::now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let status=dir.join("status.json");
        let lock=PathBuf::from(format!("{}.lock",status.display()));
        let pid=std::process::id();
        let started_at=crate::process::start_ms(pid).unwrap();
        fs::write(&lock,serde_json::json!({"pid":pid,"pidStartedAt":started_at,"ownerPid":0,"startedAt":1}).to_string()).unwrap();
        let error=HelperLock::acquire(&status,0,&dir.join("control.json")).err().unwrap();
        assert!(error.contains("Helper already running"));
        assert!(lock.is_file());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn lock_ownership_requires_exact_process_start_identity() {
        let pid=std::process::id();
        let started_at=crate::process::start_ms(pid).unwrap();
        let record=serde_json::json!({"pid":pid,"pidStartedAt":started_at});
        assert!(lock_owned_by(&record,pid,started_at));
        assert!(!lock_owned_by(&record,pid,started_at-1));
        assert!(process_identity_alive(&record));
        assert!(!process_identity_alive(&serde_json::json!({"pid":pid,"pidStartedAt":started_at-1})));
    }
}
