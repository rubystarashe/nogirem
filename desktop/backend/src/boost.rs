use crate::{
    Result, affinity, memory, muo, nic, process, storage, topology,
    workers::{Environment, read, remove},
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Each worker has its own operation lock. Status reads never wait for a long
/// reconfiguration, and affinity and memory startup can proceed independently.
pub struct Boost {
    pub env: Environment,
    affinity_operation: Mutex<()>,
    memory_operation: Mutex<()>,
    nic_cache: Mutex<(u64, Value)>,
    pub desired: AtomicBool,
    pub exiting: AtomicBool,
    pub close_pending: AtomicBool,
}
pub fn fresh(status: &Value, interval: u64, minimum: u64, now: u64) -> bool {
    status["updatedAt"].as_u64().is_some_and(|at| {
        at > 0 && now.saturating_sub(at) < interval.saturating_mul(4).max(minimum)
    })
}
fn helper_alive(status: &Value) -> bool {
    status["running"] != true
        || status["helperPid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok())
            .is_some_and(|pid| {
                process::alive(pid)
                    && status["helperStartedAt"].as_i64() == process::start_ms(pid).ok()
            })
}
fn legacy_helper(status: &Value, interval: u64, now: u64) -> Option<(u32, i64)> {
    if status["running"] != true
        || status["helperStartedAt"].is_number()
        || !fresh(status, interval, 30000, now)
    {
        return None;
    }
    let pid = u32::try_from(status["helperPid"].as_u64()?).ok()?;
    let started = process::start_ms(pid).ok()?;
    process::alive(pid).then_some((pid, started))
}
fn error(status: &Value, fallback: &str) -> String {
    status["error"]["message"]
        .as_str()
        .or(status["error"].as_str())
        .unwrap_or(fallback)
        .into()
}
fn topology_failure(detail: &str) -> String {
    if detail.contains("does not match the process affinity group") {
        return "Windows CPU 정보와 현재 논리 프로세서 구성이 일치하지 않습니다".into();
    }
    if detail.contains("size query failed") || detail.contains("GetSystemCpuSetInformation failed")
    {
        return regex::Regex::new(r"Win32 (\d+)")
            .unwrap()
            .captures(detail)
            .map(|c| format!("Windows CPU 정보 조회에 실패했습니다 (오류 코드 {})", &c[1]))
            .unwrap_or_else(|| "Windows CPU 정보 조회에 실패했습니다".into());
    }
    if detail.contains("returned no CPU Sets") {
        return "Windows에서 사용할 수 있는 CPU 코어 정보를 반환하지 않았습니다".into();
    }
    if detail.contains("malformed data") {
        return "Windows가 올바르지 않은 CPU 코어 정보를 반환했습니다".into();
    }
    if detail.contains("Inconsistent efficiency class") {
        return "Windows CPU 성능 코어 분류 정보가 일관되지 않습니다".into();
    }
    if detail.is_empty() {
        "CPU 코어 구성 분석에 실패했습니다".into()
    } else {
        format!("CPU 코어 구성 분석에 실패했습니다: {detail}")
    }
}
impl Boost {
    pub fn new(env: Environment) -> Self {
        Self {
            env,
            affinity_operation: Mutex::new(()),
            memory_operation: Mutex::new(()),
            nic_cache: Mutex::new((0, Value::Null)),
            desired: AtomicBool::new(true),
            exiting: AtomicBool::new(false),
            close_pending: AtomicBool::new(false),
        }
    }
    fn directory(&self, kind: &str) -> PathBuf {
        self.env.user.join(kind)
    }
    fn raw(&self, kind: &str) -> Value {
        read(&self.directory(kind).join("status.json"))
    }
    fn control(&self, kind: &str, mut value: Value) -> Result<()> {
        value["requestedAt"] = json!(crate::now_ms());
        storage::write_json(&self.directory(kind).join("control.json"), &value)
    }
    pub fn memory_runtime(&self) -> Value {
        let mut status = self.raw("memory");
        if !status.is_object() {
            status = json!({});
        }
        let config = read(&self.env.root.join("config.json"));
        let running = status["running"] == true
            && fresh(
                &status,
                config["memoryCleaner"]["pollIntervalMs"]
                    .as_u64()
                    .unwrap_or(1000),
                15000,
                crate::now_ms(),
            );
        status["running"] = json!(running);
        status["gameActive"] = json!(running && status["gameActive"] == true);
        status
    }
    pub fn memory_status(&self) -> Result<Value> {
        let runtime = self.memory_runtime();
        if runtime["running"] == true {
            return Ok(runtime);
        }
        let config = read(&self.env.root.join("config.json"));
        let snapshot = memory::snapshot()?;
        let mut status =
            memory::Cleaner::new(&config["memoryCleaner"], snapshot.total).status(snapshot);
        status["running"] = json!(false);
        status["gameActive"] = json!(false);
        status["error"] = runtime["error"].clone();
        Ok(status)
    }
    pub fn core_setting(&self) -> Result<Value> {
        let requested = self.env.core_count();
        let allocation = topology::resolve(requested)?;
        let detail = allocation
            .topology_error
            .as_ref()
            .and_then(|e| e["message"].as_str());
        Ok(
            json!({"supported":allocation.game_core_count.is_some()&&allocation.max_game_core_count.is_some(),"gameCoreCount":allocation.game_core_count,"defaultGameCoreCount":allocation.default_game_core_count,"maxGameCoreCount":allocation.max_game_core_count,"physicalCoreCount":allocation.physical_core_count,"performanceCoreCount":allocation.performance_core_count,"efficiencyCoreCount":allocation.efficiency_core_count,"hybrid":allocation.hybrid,"customized":requested.is_some(),"failureReason":detail.map(topology_failure),"failureDetail":detail}),
        )
    }
    pub fn simplification(&self) -> Value {
        match muo::latest(&self.env.documents.join("마비노기/설정")) {
            Ok(v) => {
                json!({"applied":v["applied"],"value":v["value"],"fileName":v["fileName"],"modifiedAt":v["modifiedAt"],"error":null})
            }
            Err(e) => {
                json!({"applied":false,"value":null,"fileName":null,"modifiedAt":null,"error":{"message":e}})
            }
        }
    }
    pub fn affinity_runtime(&self) -> Result<Value> {
        let status = self.raw("affinity");
        let config = read(&self.env.root.join("config.json"));
        let fresh = helper_alive(&status)
            && fresh(
                &status,
                config["pollIntervalMs"].as_u64().unwrap_or(5000),
                30000,
                crate::now_ms(),
            );
        let count =
            unsafe { windows_sys::Win32::System::Threading::GetActiveProcessorCount(0xffff) };
        let field = |name: &str| {
            if fresh {
                status[name].clone()
            } else {
                Value::Null
            }
        };
        let simplification = if fresh && status["characterSimplification"]["applied"] == true {
            status["characterSimplification"].clone()
        } else {
            self.simplification()
        };
        Ok(
            json!({"running":fresh&&status["running"]==true,"gameActive":fresh&&status["gameActive"]==true,"gameExecutablePath":self.env.game(),"includeNic":status["includeNic"]==true,"nicManaged":status["nicManaged"]==true,"backgroundCpuRange":status["backgroundCpuRange"].as_str().map(str::to_owned).unwrap_or_else(||format!("0-{}",count/2-1)),"gameCpuRange":status["gameCpuRange"].as_str().map(str::to_owned).unwrap_or_else(||format!("{}-{}",count/2,count-1)),"cpuTopology":field("cpuTopology"),"gameCoreSetting":self.core_setting()?,"cpuReorder":field("cpuReorder"),"gameCoreReconfigure":field("gameCoreReconfigure"),"error":status["error"],"nicStatus":status["nicStatus"],"renderer":if fresh&&status["renderer"].is_object(){status["renderer"].clone()}else{json!({"mode":"not-running","version":null})},"characterSimplification":simplification,"conflictingPrograms":status["conflictingPrograms"].as_array().cloned().unwrap_or_default()}),
        )
    }
    fn cached_nic(&self, refresh: bool, runtime: &Value) -> Value {
        let mut cache = self.nic_cache.lock().unwrap();
        if !refresh && !runtime["nicStatus"].is_null() {
            *cache = (crate::now_ms(), runtime["nicStatus"].clone());
        }
        if refresh || cache.1.is_null() || crate::now_ms().saturating_sub(cache.0) >= 30000 {
            *cache = (
                crate::now_ms(),
                nic::status()
                    .unwrap_or_else(|e| json!({"supported":false,"optimized":false,"reason":e})),
            );
        }
        cache.1.clone()
    }
    pub fn affinity_status(&self, refresh: bool) -> Result<Value> {
        let mut runtime = self.affinity_runtime()?;
        let nic = self.cached_nic(refresh, &runtime);
        if runtime["running"] == false
            && nic["optimized"] == true
            && runtime["error"]["message"]
                .as_str()
                .is_some_and(|s| s.starts_with("NIC RSS affinity"))
        {
            runtime["error"] = Value::Null;
        }
        runtime["nic"] = nic;
        Ok(runtime)
    }
    fn launch(&self, kind: &str, mut args: Vec<String>, timeout: u64) -> Result<()> {
        let directory = self.directory(kind);
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        remove(&directory.join("status.json"))?;
        remove(&directory.join("control.json"))?;
        args.extend([
            format!("--status-path={}", directory.join("status.json").display()),
            format!(
                "--control-path={}",
                directory.join("control.json").display()
            ),
        ]);
        use std::os::windows::process::CommandExt;
        let mut child = Command::new(&self.env.exe)
            .args(args)
            .current_dir(&self.env.root)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_millis(timeout);
        while Instant::now() < deadline {
            let status = self.raw(kind);
            if !status["error"].is_null() {
                let _ = self.control(kind, json!({"command":"stop","reason":"start-error"}));
                return Err(error(&status, "최적화 시작 실패"));
            }
            if status["running"] == true {
                return Ok(());
            }
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Err(format!("{kind} helper가 시작 중 종료되었습니다"));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        self.control(kind, json!({"command":"stop","reason":"start-timeout"}))?;
        Err(format!(
            "{kind} helper가 제한 시간 안에 시작되지 않았습니다"
        ))
    }
    fn launch_affinity(&self, include_nic: bool, managed: bool) -> Result<()> {
        let previous = self.raw("affinity");
        let interval = read(&self.env.root.join("config.json"))["pollIntervalMs"]
            .as_u64()
            .unwrap_or(5000);
        if let Some((pid, started)) = legacy_helper(&previous, interval, crate::now_ms()) {
            self.control(
                "affinity",
                json!({"command":"stop","reason":"version-migration"}),
            )?;
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline
                && process::alive(pid)
                && process::start_ms(pid).ok() == Some(started)
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            if process::alive(pid) && process::start_ms(pid).ok() == Some(started) {
                return Err("이전 버전 Affinity helper를 종료하지 못했습니다".into());
            }
        }
        let directory = self.directory("affinity");
        let game = self.env.user.join("game/path.json");
        std::fs::create_dir_all(game.parent().unwrap()).map_err(|e| e.to_string())?;
        let setting = self.core_setting()?;
        let mut args = vec![
            "--native-affinity-helper".into(),
            format!(
                "--affinity-state-path={}",
                directory.join("runtime-state.json").display()
            ),
            format!(
                "--applied-marker-path={}",
                directory.join("applied-marker.json").display()
            ),
            format!("--game-path-state-path={}", game.display()),
            format!("--include-nic={include_nic}"),
            format!("--nic-managed={managed}"),
        ];
        if setting["supported"] == true {
            args.push(format!("--game-core-count={}", setting["gameCoreCount"]));
        }
        self.launch("affinity", args, 45000)
    }
    fn stop_worker(&self, kind: &str, reset: bool) -> Result<()> {
        self.control(kind, json!({"command":if reset{"reset"}else{"stop"}}))?;
        let deadline =
            Instant::now() + Duration::from_secs(if kind == "affinity" { 45 } else { 15 });
        while Instant::now() < deadline {
            let status = self.raw(kind);
            if status["running"] == false {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(format!(
            "{kind} helper가 제한 시간 안에 종료되지 않았습니다"
        ))
    }
    pub fn set_memory(&self, enabled: bool, purge: bool) -> Result<Value> {
        let _operation = self.memory_operation.lock().unwrap();
        let current = self.memory_runtime();
        if enabled && current["running"] != true {
            self.launch(
                "memory",
                vec![
                    "--native-memory-helper".into(),
                    format!("--parent-pid={}", std::process::id()),
                    format!("--purge-on-start={purge}"),
                ],
                15000,
            )?;
        }
        if !enabled && current["running"] == true {
            self.stop_worker("memory", false)?;
        }
        self.memory_status()
    }
    pub fn set_affinity(&self, enabled: bool, include_nic: bool, reset: bool) -> Result<Value> {
        let _operation = self.affinity_operation.lock().unwrap();
        let current = self.affinity_runtime()?;
        if (enabled || reset) && current["running"] != true {
            self.launch_affinity(include_nic, current["nicManaged"] == true)?;
        }
        if !enabled && (current["running"] == true || reset) {
            self.stop_worker("affinity", reset)?;
        }
        if current["nicManaged"] == true {
            self.nic_cache.lock().unwrap().0 = 0;
        }
        self.affinity_status(false)
    }
    pub fn set_enabled(&self, enabled: bool, include_nic: bool) -> Result<Value> {
        let old = self.desired.swap(enabled, Ordering::SeqCst);
        let (affinity, memory) = std::thread::scope(|scope| {
            let a = scope.spawn(|| self.set_affinity(enabled, include_nic, !enabled));
            let m = scope.spawn(|| self.set_memory(enabled, false));
            (a.join().unwrap(), m.join().unwrap())
        });
        match (affinity, memory) {
            (Ok(a), Ok(m)) => Ok(json!({"affinity":a,"memory":m})),
            (a, m) => {
                self.desired.store(old, Ordering::SeqCst);
                Err(a.err().or(m.err()).unwrap())
            }
        }
    }
    pub fn ensure_affinity(&self) -> Result<Value> {
        let current = self.affinity_runtime()?;
        if current["running"] != true
            && self.desired.load(Ordering::SeqCst)
            && !self.exiting.load(Ordering::SeqCst)
            && !self.close_pending.load(Ordering::SeqCst)
        {
            self.set_affinity(true, false, false)
        } else {
            Ok(current)
        }
    }
    pub fn ensure_memory(&self) -> Result<Value> {
        let current = self.memory_runtime();
        if current["running"] != true
            && self.desired.load(Ordering::SeqCst)
            && !self.exiting.load(Ordering::SeqCst)
            && !self.close_pending.load(Ordering::SeqCst)
        {
            self.set_memory(true, false)
        } else {
            Ok(current)
        }
    }
    fn wait_request(&self, field: &str, id: &str, timeout: u64) -> Result<()> {
        let deadline = Instant::now() + Duration::from_millis(timeout);
        loop {
            let status = self.raw("affinity");
            if status["running"] == false {
                return Err("Affinity helper가 작업 중 종료되었습니다".into());
            }
            if status[field]["requestId"] == id {
                match status[field]["state"].as_str() {
                    Some("completed") => return Ok(()),
                    Some("failed") => return Err(error(&status[field], "CPU 작업에 실패했습니다")),
                    _ => {}
                }
            }
            if Instant::now() >= deadline {
                return Err(
                    if field == "cpuReorder" && status[field]["requestId"] == id {
                        "CPU 재정렬 원상복구 확인이 지연되고 있습니다"
                    } else {
                        "CPU 설정이 제한 시간 안에 적용되지 않았습니다"
                    }
                    .into(),
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    pub fn set_core_count(&self, value: &Value) -> Result<Value> {
        let _operation = self.affinity_operation.lock().unwrap();
        let setting = self.core_setting()?;
        if setting["supported"] != true {
            return Err("이 PC에서는 물리 CPU 코어 구성을 확인할 수 없습니다".into());
        }
        let parsed = value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(f64::NAN);
        let max = setting["maxGameCoreCount"].as_u64().unwrap_or(0);
        if !parsed.is_finite() || parsed.fract() != 0. || parsed < 1. || parsed > max as f64 {
            return Err(format!("마비노기 CPU 코어 수는 1~{max} 사이여야 합니다"));
        }
        storage::write_json(
            &self.directory("affinity").join("game-core-setting.json"),
            &json!({"gameCoreCount":parsed as u64,"updatedAt":crate::app_services::iso()}),
        )?;
        let current = self.affinity_runtime()?;
        if current["running"] != true
            || current["cpuTopology"]["gameCoreCount"].as_u64() == Some(parsed as u64)
        {
            return self.affinity_status(false);
        }
        if current["cpuReorder"]["state"] == "running" {
            return Err("CPU 재정렬이 끝난 뒤 코어 수를 변경해 주세요".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.control(
            "affinity",
            json!({"command":"set-game-core-count","requestId":id,"gameCoreCount":parsed as u64}),
        )?;
        self.wait_request("gameCoreReconfigure", &id, 10000)?;
        self.affinity_status(false)
    }
    pub fn reorder(&self) -> Result<Value> {
        let _operation = self.affinity_operation.lock().unwrap();
        if self.exiting.load(Ordering::SeqCst) || self.close_pending.load(Ordering::SeqCst) {
            return Err("앱 종료 처리 중에는 CPU 재정렬을 실행할 수 없습니다".into());
        }
        let current = self.affinity_runtime()?;
        if !self.desired.load(Ordering::SeqCst)
            || current["running"] != true
            || self.memory_runtime()["running"] != true
        {
            return Err("실시간 부스트가 켜져 있을 때만 CPU 재정렬을 실행할 수 있습니다".into());
        }
        if current["gameActive"] != true {
            return Err("실행 중인 마비노기를 찾을 수 없습니다".into());
        }
        if current["cpuReorder"]["state"] == "running" {
            return Err("CPU 재정렬이 이미 실행 중입니다".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.control("affinity", json!({"command":"cpu-reorder","requestId":id}))?;
        self.wait_request("cpuReorder", &id, 45000)?;
        self.affinity_runtime()
    }
    pub fn exit_confirmation(&self) -> Result<bool> {
        let directory = self.directory("affinity");
        let state =
            storage::read_json(&directory.join("runtime-state.json"))?.unwrap_or(Value::Null);
        let marker =
            storage::read_json(&directory.join("applied-marker.json"))?.unwrap_or(Value::Null);
        let mut entries = vec![];
        for value in [&state, &marker] {
            if value["entries"].is_array() {
                entries.extend(
                    serde_json::from_value::<Vec<affinity::Entry>>(value["entries"].clone())
                        .map_err(|e| e.to_string())?,
                );
            }
        }
        let needed = affinity::has_live_entries(&entries)
            || marker["nicManaged"] == true
            || self.raw("affinity")["nicManaged"] == true;
        if !needed {
            remove(&directory.join("applied-marker.json"))?;
        }
        Ok(needed)
    }
    pub fn stop(&self, reset: bool) -> Result<()> {
        self.exiting.store(true, Ordering::SeqCst);
        self.close_pending.store(false, Ordering::SeqCst);
        let (a, m) = std::thread::scope(|scope| {
            let a = scope.spawn(|| self.set_affinity(false, false, reset));
            let m = scope.spawn(|| self.set_memory(false, false));
            (a.join().unwrap(), m.join().unwrap())
        });
        a?;
        m?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn freshness_matches_worker_poll_intervals() {
        let v = json!({"updatedAt":1000});
        assert!(fresh(&v, 5000, 30000, 30999));
        assert!(!fresh(&v, 5000, 30000, 31000));
        assert!(!fresh(&Value::Null, 1000, 15000, 100));
    }
    #[test]
    fn running_affinity_status_requires_a_live_helper() {
        let pid = std::process::id();
        let started = process::start_ms(pid).unwrap();
        assert!(helper_alive(
            &json!({"running":true,"helperPid":pid,"helperStartedAt":started})
        ));
        assert!(!helper_alive(
            &json!({"running":true,"helperPid":pid,"helperStartedAt":started - 1})
        ));
        assert!(!helper_alive(&json!({"running":true})));
        assert!(helper_alive(&json!({"running":false})));
    }
    #[test]
    fn legacy_affinity_helper_requires_a_fresh_live_process() {
        let pid = std::process::id();
        let status = json!({"running":true,"helperPid":pid,"updatedAt":1000});
        assert_eq!(
            legacy_helper(&status, 5000, 2000),
            Some((pid, process::start_ms(pid).unwrap()))
        );
        assert!(legacy_helper(&status, 5000, 31000).is_none());
        assert!(
            legacy_helper(
                &json!({"running":true,"helperPid":pid,"helperStartedAt":1,"updatedAt":1000}),
                5000,
                2000
            )
            .is_none()
        );
    }
    #[test]
    fn topology_errors_are_user_visible() {
        assert_eq!(
            topology_failure("GetSystemCpuSetInformation failed Win32 5"),
            "Windows CPU 정보 조회에 실패했습니다 (오류 코드 5)"
        );
    }
}
