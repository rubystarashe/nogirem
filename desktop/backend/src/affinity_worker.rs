use crate::{Result, affinity::Manager, nic, process, storage, topology};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
static RENDERER_VERSION: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\bDXVK:\s*(v[^\s]+)").unwrap());
static RENDERER_MARKERS: std::sync::LazyLock<[regex::Regex; 6]> = std::sync::LazyLock::new(|| [
    r"(?i)\bCreating device:", r"(?i)\bPresenter:\s*Actual swapchain properties:",
    r"(?i)\bVulkan:\s*Found vkGetInstanceProcAddr", r"(?i)\bFound device:",
    r"(?i)\bD3D9DeviceEx::ResetSwapChain:", r"(?i)\bDevice reset\b",
].map(|pattern| regex::Regex::new(pattern).unwrap()));
fn renderer(manager: &Manager, executable: &str) -> Value {
    if !manager.game_active {
        return json!({"mode":"not-running","version":null});
    }
    let started = manager
        .latest_game_start_time
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis());
    let Some(started) = started else {
        return json!({"mode":"detecting","version":null});
    };
    let log = Path::new(executable)
        .parent()
        .unwrap_or(Path::new(""))
        .join("Client_d3d9.log");
    match std::fs::read_to_string(&log) {
        Ok(text) => {
            let recent = std::fs::metadata(&log)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .is_some_and(|t| t.as_millis() as i64 >= started - 5000);
            let version = RENDERER_VERSION.captures(&text).map(|c| c[1].to_owned());
            let has = |index: usize| RENDERER_MARKERS[index].is_match(&text);
            let initialized = (has(0) && has(1)) || (has(2) && has(3) && has(4) && has(5));
            if recent && initialized && version.is_some() {
                return json!({"mode":"vulkan","version":version});
            }
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return json!({"mode":"detecting","version":null});
        }
        Err(_) => (),
    }
    json!({"mode":if crate::now_ms() as i64-started>=15000{"direct3d9"}else{"detecting"},"version":null})
}
fn cpu_info(manager: &Manager) -> Value {
    let a = &manager.allocation;
    json!({"source":a.source,"hybrid":a.hybrid,"physicalCoreCount":a.physical_core_count,"performanceCoreCount":a.performance_core_count,"efficiencyCoreCount":a.efficiency_core_count,"gameCoreCount":a.game_core_count,"defaultGameCoreCount":a.default_game_core_count,"maxGameCoreCount":a.max_game_core_count,"backgroundCpuRange":topology::cpu_range(&a.background_cpu_indexes),"gameCpuRange":topology::cpu_range(&a.game_cpu_indexes)})
}
struct Status {
    path: PathBuf,
    game_path: PathBuf,
    marker_path: PathBuf,
    executable: String,
    include_nic: bool,
    nic_managed: bool,
    nic_status: Value,
    marker: Value,
    reorder: Value,
    reconfigure: Value,
    failure: Option<String>,
}
impl Status {
    fn record(&mut self, manager: &Manager) -> Result<()> {
        let mut entries: BTreeMap<String, Value> = self.marker["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| (format!("{}:{}", e["pid"], e["startTime"]), e.clone()))
            .collect();
        for entry in manager.entries() {
            let e = serde_json::to_value(entry).map_err(|e| e.to_string())?;
            entries.insert(format!("{}:{}", e["pid"], e["startTime"]), e);
        }
        if entries.is_empty() && !self.nic_managed {
            return Ok(());
        }
        self.marker = json!({"appliedAt":self.marker["appliedAt"].as_u64().unwrap_or(crate::now_ms()),"helperPid":std::process::id(),"nicManaged":self.nic_managed||self.marker["nicManaged"]==true,"entries":entries.into_values().collect::<Vec<_>>()});
        storage::write_json(&self.marker_path, &self.marker)
    }
    fn write(&mut self, manager: &Manager, running: bool, exit: Option<&str>) -> Result<()> {
        if let Some(executable) = manager.latest_game_executable_path.as_ref() {
            let saved = storage::read_json(&self.game_path)?.unwrap_or(Value::Null);
            if saved["executablePath"].as_str() != Some(executable) {
                storage::write_json(
                    &self.game_path,
                    &json!({"executablePath":executable,"detectedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"source":"running-process"}),
                )?;
            }
            self.executable = executable.clone();
        }
        let topology = cpu_info(manager);
        let mut pinned = topology.clone();
        pinned["mask"] = json!(format!("0x{:x}", manager.allocation.background_mask));
        pinned["cpuRange"] = topology["backgroundCpuRange"].clone();
        let mut conflicts = Vec::new();
        for (name, exes) in [
            (
                "ISLC",
                vec!["islc.exe", "intelligent standby list cleaner islc.exe"],
            ),
            (
                "Process Lasso",
                vec!["processlasso.exe", "processgovernor.exe"],
            ),
        ] {
            if exes
                .iter()
                .any(|n| manager.running_process_names.contains(*n))
            {
                conflicts.push(name);
            }
        }
        let mut value = json!({"running":running,"gameActive":running&&manager.game_active,"gameExecutablePath":self.executable,"renderer":renderer(manager,&self.executable),"includeNic":self.include_nic,"nicManaged":self.nic_managed,"backgroundCpuRange":topology["backgroundCpuRange"],"gameCpuRange":topology["gameCpuRange"],"cpuTopology":topology,"helperPid":std::process::id(),"helperAffinity":pinned,"nicStatus":self.nic_status,"appliedMarkerRecorded":!self.marker.is_null(),"conflictingPrograms":conflicts,"cpuReorder":self.reorder,"gameCoreReconfigure":self.reconfigure,"updatedAt":crate::now_ms(),"error":self.failure.as_ref().map(|e|json!({"message":e}))});
        if let Some(exit) = exit {
            value["exitAction"] = json!(exit);
            value["stoppedAt"] = json!(crate::now_ms());
        }
        storage::write_json(&self.path, &value)
    }
}
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let arg = |name: &str| {
        args.iter()
            .find_map(|s| s.strip_prefix(&format!("--{name}=")))
            .map(str::to_owned)
    };
    let path = |name: &str| {
        arg(name)
            .map(PathBuf::from)
            .ok_or_else(|| format!("Affinity helper 경로가 없습니다: {name}"))
    };
    let status_path = path("status-path")?;
    let control = path("control-path")?;
    let state = path("affinity-state-path")?;
    let game_path = path("game-path-state-path")?;
    let marker_path = path("applied-marker-path")?;
    let _lock = storage::HelperLock::acquire(&status_path, 0, &control)?;
    let config = storage::read_json(&root.join("config.json"))?.ok_or("config.json missing")?;
    let requested = arg("game-core-count").and_then(|s| s.parse::<i64>().ok());
    let mut manager = Manager::new(
        config.clone(),
        state.clone(),
        true,
        true,
        requested,
        false,
        false,
    )?;
    process::set_affinity(std::process::id(), manager.allocation.background_mask)?;
    match std::fs::remove_file(&state) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.to_string()),
    }
    let saved = storage::read_json(&game_path)?.unwrap_or(Value::Null);
    let previous = storage::read_json(&status_path)?.unwrap_or(Value::Null);
    let marker = storage::read_json(&marker_path)?.unwrap_or(Value::Null);
    let mut status = Status {
        path: status_path,
        game_path,
        marker_path,
        executable: saved["executablePath"]
            .as_str()
            .or(previous["gameExecutablePath"].as_str())
            .or(config["gameExecutable"].as_str())
            .unwrap_or("")
            .into(),
        include_nic: arg("include-nic").as_deref() == Some("true"),
        nic_managed: arg("nic-managed").as_deref() == Some("true"),
        nic_status: Value::Null,
        marker,
        reorder: Value::Null,
        reconfigure: Value::Null,
        failure: None,
    };
    let mut exit = "keep";
    let outcome = (|| -> Result<()> {
        if status.include_nic {
            status.nic_status = nic::apply(true)?;
            status.nic_managed |= status.nic_status["applied"] == true;
            status.record(&manager)?;
        }
        status.write(&manager, true, None)?;
        loop {
            if let Some(command) = storage::read_json(&control)? {
                std::fs::remove_file(&control).map_err(|e| e.to_string())?;
                match command["command"].as_str().unwrap_or("") {
                    "set-game-core-count" => {
                        status.reconfigure = json!({"requestId":command["requestId"],"state":"running","requestedAt":command["requestedAt"]});
                        status.write(&manager, true, None)?;
                        let result = (|| -> Result<()> {
                            let requested = command["gameCoreCount"]
                                .as_i64()
                                .ok_or("마비노기 CPU 코어 수가 올바르지 않습니다")?;
                            manager.reset_all()?;
                            manager = Manager::new(
                                config.clone(),
                                state.clone(),
                                true,
                                true,
                                Some(requested),
                                false,
                                false,
                            )?;
                            process::set_affinity(
                                std::process::id(),
                                manager.allocation.background_mask,
                            )?;
                            manager.tick()
                        })();
                        status.reconfigure["state"] = json!(if result.is_ok() {
                            "completed"
                        } else {
                            "failed"
                        });
                        status.reconfigure["completedAt"] = json!(crate::now_ms());
                        if let Err(e) = result {
                            status.reconfigure["error"] = json!({"message":e});
                        }
                    }
                    "cpu-reorder" => {
                        status.reorder = json!({"requestId":command["requestId"],"state":"running","startedAt":crate::now_ms(),"endsAt":crate::now_ms()+3000,"error":null});
                        status.write(&manager, true, None)?;
                        let result = manager.reorder();
                        status.reorder["state"] = json!(if result.is_ok() {
                            "completed"
                        } else {
                            "failed"
                        });
                        status.reorder["completedAt"] = json!(crate::now_ms());
                        match result {
                            Ok(v) => status.reorder["result"] = v,
                            Err(e) => status.reorder["error"] = json!({"message":e}),
                        }
                    }
                    command => {
                        exit = if command == "reset" { "reset" } else { "keep" };
                        break;
                    }
                }
            }
            manager.tick()?;
            if let Err(e) = status.record(&manager) {
                eprintln!("Affinity 적용 기록 실패: {e}");
            }
            if let Err(e) = status.write(&manager, true, None) {
                eprintln!("Affinity 상태 기록 실패: {e}");
            }
            let deadline = Instant::now()
                + Duration::from_millis(config["pollIntervalMs"].as_u64().unwrap_or(5000));
            while Instant::now() < deadline && !control.exists() {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Ok(())
    })();
    if let Err(e) = &outcome {
        status.failure = Some(e.clone());
    }
    let cleanup = if exit == "reset" {
        manager.reset_all().map(|_| ())
    } else {
        manager.stop_without_restore()
    };
    if let Err(e) = cleanup {
        status.failure.get_or_insert(e);
    }
    if exit == "reset" && status.nic_managed {
        match nic::restore(true) {
            Ok(v) => {
                status.nic_managed = false;
                status.nic_status = v;
            }
            Err(e) => {
                status.failure.get_or_insert(e);
            }
        }
    }
    if exit == "reset" && status.failure.is_none() {
        let _ = std::fs::remove_file(&status.marker_path);
        status.marker = Value::Null;
    }
    status.write(&manager, false, Some(exit))?;
    if let Some(error) = status.failure {
        Err(error)
    } else {
        outcome
    }
}
