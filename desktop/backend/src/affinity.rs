use crate::{
    Result,
    process::{self, Process},
    storage,
    topology::{self, Allocation},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};
const EVENT_LOG_MAX_BYTES: u64 = 1024 * 1024;

fn append_event(path: &PathBuf, event: &str, details: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() >= EVENT_LOG_MAX_BYTES) {
        let previous = path.with_extension("log.previous");
        match std::fs::remove_file(&previous) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
        std::fs::rename(path, previous).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let record = json!({
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "event": event,
        "details": details
    });
    writeln!(
        file,
        "{}",
        serde_json::to_string(&record).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.sync_data().map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub pid: u32,
    pub start_time: Option<String>,
    pub name: String,
    pub original_mask: String,
    pub applied_mask: String,
    pub role: String,
}
fn mask(s: &str) -> Result<u64> {
    if let Some(n) = s.strip_prefix("0x") {
        u64::from_str_radix(n, 16)
    } else {
        s.parse()
    }
    .map_err(|e| e.to_string())
}
fn key(p: &Process) -> String {
    format!("{}:{}", p.pid, p.start_time.as_deref().unwrap_or("null"))
}
fn names(v: &Value) -> HashSet<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_lowercase)
        .collect()
}
fn patterns(v: &Value) -> Result<Vec<Regex>> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|p| Regex::new(&format!("(?i){p}")).map_err(|e| e.to_string()))
        .collect()
}
pub struct Manager {
    pub allocation: Allocation,
    pub game_active: bool,
    pub latest_game_start_time: Option<String>,
    pub latest_game_executable_path: Option<String>,
    pub running_process_names: HashSet<String>,
    config: Value,
    state_path: PathBuf,
    event_path: PathBuf,
    apply_changes: bool,
    restore_on_exit: bool,
    game_mask: Option<u64>,
    background_mask: u64,
    latency_mask: u64,
    current_session: Option<u32>,
    exclude_names: HashSet<String>,
    exclude_patterns: Vec<Regex>,
    latency_names: HashSet<String>,
    latency_patterns: Vec<Regex>,
    changed: BTreeMap<String, Entry>,
    handled: HashSet<String>,
    normal_mode: bool,
}
impl Manager {
    pub fn new(
        config: Value,
        state_path: PathBuf,
        apply_changes: bool,
        restore_on_exit: bool,
        requested: Option<i64>,
        last_core: bool,
        passive: bool,
    ) -> Result<Self> {
        let allocation = topology::resolve(requested)?;
        let game_mask = if passive {
            None
        } else {
            Some(if last_core {
                allocation.last_performance_core_mask
            } else {
                allocation.game_mask
            })
        };
        let background_mask = if passive {
            allocation.game_mask
        } else if last_core {
            allocation.all_mask ^ game_mask.unwrap()
        } else {
            allocation.background_mask
        };
        let latency_mask = if allocation.alternate_game_mask == 0 {
            background_mask
        } else {
            allocation.alternate_game_mask
        };
        let event_path = state_path.with_file_name("events.log");
        let manager = Self {
            exclude_names: names(&config["excludeNames"]),
            exclude_patterns: patterns(&config["excludeNamePatterns"])?,
            latency_names: names(&config["latencySensitiveNames"]),
            latency_patterns: patterns(&config["latencySensitiveNamePatterns"])?,
            allocation,
            config,
            state_path,
            event_path,
            apply_changes,
            restore_on_exit,
            game_mask,
            background_mask,
            latency_mask,
            current_session: process::session(std::process::id()),
            game_active: false,
            latest_game_start_time: None,
            latest_game_executable_path: None,
            running_process_names: HashSet::new(),
            changed: BTreeMap::new(),
            handled: HashSet::new(),
            normal_mode: !last_core && !passive,
        };
        manager.event(
            "helper-created",
            json!({
                "applyChanges": apply_changes,
                "restoreOnExit": restore_on_exit,
                "requestedGameCoreCount": requested,
                "gameMask": manager.game_mask.map(|value| format!("0x{value:x}")),
                "backgroundMask": format!("0x{:x}", manager.background_mask),
                "latencyMask": format!("0x{:x}", manager.latency_mask),
                "allocationSource": manager.allocation.source
            }),
        );
        Ok(manager)
    }
    fn event(&self, event: &str, details: Value) {
        if let Err(error) = append_event(&self.event_path, event, details) {
            eprintln!("[affinity-event] {event}: {error}");
        }
    }
    fn conflicts(&self) -> Vec<&'static str> {
        let mut conflicts = Vec::new();
        if ["islc.exe", "intelligent standby list cleaner islc.exe"]
            .iter()
            .any(|name| self.running_process_names.contains(*name))
        {
            conflicts.push("ISLC");
        }
        if ["processlasso.exe", "processgovernor.exe"]
            .iter()
            .any(|name| self.running_process_names.contains(*name))
        {
            conflicts.push("Process Lasso");
        }
        conflicts
    }
    fn eligible(&self, p: &Process) -> bool {
        let name = p.name.to_lowercase();
        let path = p
            .path
            .as_deref()
            .unwrap_or("")
            .replace('/', "\\")
            .to_lowercase();
        p.pid > 4
            && p.pid != std::process::id()
            && p.session_id == self.current_session
            && !path.is_empty()
            && !path.starts_with("c:\\windows\\")
            && !self.exclude_names.contains(&name)
            && !self.exclude_patterns.iter().any(|r| r.is_match(&name))
            && !process::matches_game(p, &self.config)
    }
    fn latency(&self, p: &Process) -> bool {
        self.eligible(p)
            && (self.latency_names.contains(&p.name.to_lowercase())
                || self.latency_patterns.iter().any(|r| r.is_match(&p.name)))
    }
    fn apply_one(&mut self, p: &Process, target: u64, role: &str) -> Result<()> {
        let key = key(p);
        if self.handled.contains(&key) {
            return Ok(());
        }
        let result = (|| {
            let original = process::affinity(p.pid)?;
            if original == target {
                return Ok(());
            }
            if !self.apply_changes {
                return Ok(());
            }
            // Avoid acting on a reused PID after enumeration.
            if process::start_time(p.pid) != p.start_time {
                return Err("Process identity changed".into());
            }
            process::set_affinity(p.pid, target)?;
            self.changed.insert(
                key.clone(),
                Entry {
                    pid: p.pid,
                    start_time: p.start_time.clone(),
                    name: p.name.clone(),
                    original_mask: format!("0x{original:x}"),
                    applied_mask: format!("0x{target:x}"),
                    role: role.into(),
                },
            );
            self.save()
        })();
        self.handled.insert(key);
        result
    }
    fn save(&self) -> Result<()> {
        storage::write_json(&self.state_path, &json!({"entries":self.entries()}))
    }
    pub fn entries(&self) -> Vec<Entry> {
        self.changed.values().cloned().collect()
    }
    fn apply_all(&mut self, processes: &[Process]) -> Value {
        let before = self.changed.len();
        let (mut targeted, mut failed) = (0, 0);
        for p in processes {
            let target = if process::matches_game(p, &self.config) {
                self.game_mask.map(|m| (m, "game"))
            } else if self.latency(p) {
                Some((self.latency_mask, "latency"))
            } else if self.eligible(p) {
                Some((self.background_mask, "background"))
            } else {
                None
            };
            if let Some((m, role)) = target {
                targeted += 1;
                if let Err(e) = self.apply_one(p, m, role) {
                    failed += 1;
                    eprintln!("[affinity-skip] {} PID={}: {e}", p.name, p.pid);
                }
            }
        }
        json!({
            "targetedCount": targeted,
            "changedCount": self.changed.len().saturating_sub(before),
            "failedCount": failed
        })
    }
    pub fn tick(&mut self) -> Result<()> {
        let processes = process::list(true)?;
        self.running_process_names = processes.iter().map(|p| p.name.to_lowercase()).collect();
        let mut games: Vec<_> = processes
            .iter()
            .filter(|p| process::matches_game(p, &self.config))
            .collect();
        if games.is_empty() {
            self.latest_game_start_time = None;
            if self.game_active {
                self.event(
                    "game-exit-detected",
                    json!({
                        "processCount": processes.len(),
                        "trackedAffinityCount": self.changed.len(),
                        "conflictingPrograms": self.conflicts()
                    }),
                );
                if self.restore_on_exit {
                    self.restore_all_with_reason("game-exit")?;
                } else {
                    self.handled.clear();
                    self.event(
                        "game-exit-without-restore",
                        json!({"reason":"restore-disabled"}),
                    );
                }
                self.game_active = false;
            }
            return Ok(());
        }
        games.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        self.latest_game_start_time = games.last().and_then(|p| p.start_time.clone());
        if let Some(path) = games.iter().rev().find_map(|p| p.path.clone()) {
            self.latest_game_executable_path = Some(path);
        }
        let starting = !self.game_active;
        let started = Instant::now();
        if starting {
            self.event(
                "game-start-detected",
                json!({
                    "gameProcesses": games.iter().map(|process| json!({
                        "pid": process.pid,
                        "name": process.name,
                        "startTime": process.start_time
                    })).collect::<Vec<_>>(),
                    "processCount": processes.len(),
                    "gameMask": self.game_mask.map(|value| format!("0x{value:x}")),
                    "backgroundMask": format!("0x{:x}", self.background_mask),
                    "latencyMask": format!("0x{:x}", self.latency_mask),
                    "conflictingPrograms": self.conflicts()
                }),
            );
        }
        self.game_active = true;
        let result = self.apply_all(&processes);
        if starting {
            self.event(
                "game-affinity-applied",
                json!({
                    "elapsedMs": started.elapsed().as_millis(),
                    "result": result
                }),
            );
        }
        Ok(())
    }
    pub fn restore_entries(&self, entries: &[Entry]) -> Result<()> {
        self.restore_entries_with_reason(entries, "explicit")
    }
    fn restore_entries_with_reason(&self, entries: &[Entry], reason: &str) -> Result<()> {
        let started = Instant::now();
        self.event(
            "affinity-restore-started",
            json!({"reason":reason,"entryCount":entries.len()}),
        );
        let live = match process::list(true) {
            Ok(live) => live,
            Err(error) => {
                self.event(
                    "affinity-restore-failed",
                    json!({"reason":reason,"stage":"process-list","error":error}),
                );
                return Err(error);
            }
        };
        let identities: HashSet<_> = live.iter().map(key).collect();
        let (mut missing, mut identity_changed, mut restored, mut failed) = (0, 0, 0, 0);
        for e in entries {
            if !identities.contains(&format!(
                "{}:{}",
                e.pid,
                e.start_time.as_deref().unwrap_or("null")
            )) {
                missing += 1;
                continue;
            }
            if self.apply_changes {
                if process::start_time(e.pid) != e.start_time {
                    identity_changed += 1;
                    continue;
                }
                match mask(&e.original_mask).and_then(|m| process::set_affinity(e.pid, m)) {
                    Ok(()) => restored += 1,
                    Err(error) => {
                        failed += 1;
                        eprintln!("[restore-skip] PID={}: {error}", e.pid);
                    }
                }
            }
        }
        self.event(
            "affinity-restore-completed",
            json!({
                "reason": reason,
                "elapsedMs": started.elapsed().as_millis(),
                "entryCount": entries.len(),
                "liveProcessCount": live.len(),
                "restoredCount": restored,
                "missingCount": missing,
                "identityChangedCount": identity_changed,
                "failedCount": failed
            }),
        );
        Ok(())
    }
    pub fn restore_all(&mut self) -> Result<()> {
        self.restore_all_with_reason("explicit-all")
    }
    fn restore_all_with_reason(&mut self, reason: &str) -> Result<()> {
        self.restore_entries_with_reason(&self.entries(), reason)?;
        self.clear()
    }
    pub fn stop_without_restore(&mut self) -> Result<()> {
        self.clear()
    }
    fn clear(&mut self) -> Result<()> {
        self.changed.clear();
        self.handled.clear();
        if self.apply_changes {
            match std::fs::remove_file(&self.state_path) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
    pub fn recover(&mut self) -> Result<()> {
        if let Some(v) = storage::read_json(&self.state_path)? {
            let entries: Vec<Entry> =
                serde_json::from_value(v["entries"].clone()).map_err(|e| e.to_string())?;
            self.restore_entries_with_reason(&entries, "startup-recovery")?;
            self.clear()?;
        }
        Ok(())
    }
    pub fn reset_all(&mut self) -> Result<Value> {
        let processes = process::list(true)?;
        let eligible: Vec<_> = processes
            .iter()
            .filter(|p| {
                p.pid > 4 && p.pid != std::process::id() && p.session_id == self.current_session
            })
            .collect();
        let result = self.force(&eligible, self.allocation.all_mask);
        self.clear()?;
        Ok(result)
    }
    fn force(&self, processes: &[&Process], target: u64) -> Value {
        let (mut changed, mut already, mut skipped) = (0, 0, 0);
        for p in processes {
            let result = process::affinity(p.pid).and_then(|current| {
                if current == target {
                    return Ok(false);
                }
                if self.apply_changes {
                    if process::start_time(p.pid) != p.start_time {
                        return Err("Process identity changed".into());
                    }
                    process::set_affinity(p.pid, target)?;
                }
                Ok(true)
            });
            match result {
                Ok(true) => changed += 1,
                Ok(false) => already += 1,
                Err(_) => skipped += 1,
            }
        }
        json!({"changedCount":changed,"alreadyCount":already,"skippedCount":skipped})
    }
    pub fn reorder(&mut self) -> Result<Value> {
        if !self.normal_mode {
            return Err("CPU 재정렬은 기본 절반 분할 모드에서만 사용할 수 있습니다".into());
        }
        let before = process::list(true)?;
        let games: Vec<_> = before
            .iter()
            .filter(|p| process::matches_game(p, &self.config))
            .collect();
        if games.is_empty() {
            return Err("실행 중인 마비노기를 찾을 수 없습니다".into());
        }
        if self.allocation.alternate_game_mask == 0 {
            return Err("CPU 재정렬에 사용할 대체 P-core 그룹이 없습니다".into());
        }
        let _ = self.apply_all(&before);
        let temporary_game = self.force(&games, self.allocation.alternate_game_mask);
        let temporary_background = self.force(
            &before
                .iter()
                .filter(|p| self.eligible(p) && !self.latency(p))
                .collect::<Vec<_>>(),
            self.allocation.alternate_background_mask,
        );
        let temporary_latency = self.force(
            &before
                .iter()
                .filter(|p| self.latency(p))
                .collect::<Vec<_>>(),
            self.allocation.game_mask,
        );
        std::thread::sleep(Duration::from_secs(3));
        // If enumeration fails, restore the known live identities before returning.
        let after = match process::list(true) {
            Ok(v) => v,
            Err(e) => {
                self.force(&games, self.allocation.game_mask);
                self.force(
                    &before
                        .iter()
                        .filter(|p| self.eligible(p) && !self.latency(p))
                        .collect::<Vec<_>>(),
                    self.background_mask,
                );
                self.force(
                    &before
                        .iter()
                        .filter(|p| self.latency(p))
                        .collect::<Vec<_>>(),
                    self.latency_mask,
                );
                return Err(e);
            }
        };
        let _ = self.apply_all(&after);
        let final_games: Vec<_> = after
            .iter()
            .filter(|p| process::matches_game(p, &self.config))
            .collect();
        let final_game = self.force(&final_games, self.allocation.game_mask);
        let final_background = self.force(
            &after
                .iter()
                .filter(|p| self.eligible(p) && !self.latency(p))
                .collect::<Vec<_>>(),
            self.background_mask,
        );
        self.force(
            &after.iter().filter(|p| self.latency(p)).collect::<Vec<_>>(),
            self.latency_mask,
        );
        self.game_active = !final_games.is_empty();
        Ok(
            json!({"durationMs":3000,"temporary":{"game":temporary_game,"background":temporary_background,"latency":temporary_latency},"final":{"game":final_game,"background":final_background}}),
        )
    }
}
pub fn has_live_entries(entries: &[Entry]) -> bool {
    entries.iter().any(|e| {
        e.start_time.is_some()
            && process::start_time(e.pid) == e.start_time
            && mask(&e.applied_mask)
                .ok()
                .zip(process::affinity(e.pid).ok())
                .is_some_and(|(a, b)| a == b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_log() -> PathBuf {
        std::env::temp_dir()
            .join(format!("nogirem-affinity-events-{}", uuid::Uuid::new_v4()))
            .join("events.log")
    }

    #[test]
    fn event_log_is_json_lines_and_flushes_transition_details() {
        let path = temporary_log();
        append_event(
            &path,
            "game-exit-detected",
            json!({"trackedAffinityCount":169}),
        )
        .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let event: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(event["event"], "game-exit-detected");
        assert_eq!(event["details"]["trackedAffinityCount"], 169);
        assert!(event["timestamp"].as_str().unwrap().ends_with('Z'));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn event_log_rotates_before_exceeding_the_bound_again() {
        let path = temporary_log();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(EVENT_LOG_MAX_BYTES).unwrap();
        append_event(&path, "helper-created", json!({})).unwrap();
        assert!(path.with_extension("log.previous").is_file());
        assert!(std::fs::metadata(&path).unwrap().len() < EVENT_LOG_MAX_BYTES);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
