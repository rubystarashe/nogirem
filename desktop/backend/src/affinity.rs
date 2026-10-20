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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_priority_class: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_priority_class: Option<u32>,
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
fn entry_key(entry: &Entry) -> String {
    format!(
        "{}:{}",
        entry.pid,
        entry.start_time.as_deref().unwrap_or("null")
    )
}
fn legacy_stale_entry(
    process: &Process,
    current: u64,
    known: &HashSet<String>,
    applied: &HashSet<u64>,
    original: u64,
) -> Option<Entry> {
    (!known.contains(&key(process)) && current != original && applied.contains(&current)).then(
        || Entry {
            pid: process.pid,
            start_time: process.start_time.clone(),
            name: process.name.clone(),
            original_mask: format!("0x{original:x}"),
            applied_mask: format!("0x{current:x}"),
            role: "legacy-reset".into(),
            original_priority_class: None,
            applied_priority_class: None,
        },
    )
}
fn legacy_process_eligible(process: &Process, current_session: Option<u32>) -> bool {
    let path=process.path.as_deref().unwrap_or("").replace('/',"\\").to_lowercase();
    process.pid>4
        && process.pid!=std::process::id()
        && process.session_id==current_session
        && !path.is_empty()
}
fn priority_target(role: &str, enabled: bool) -> Option<u32> {
    (enabled && role == "background").then_some(process::BACKGROUND_PRIORITY_CLASS)
}
fn original_if_unchanged<T: Copy + Eq>(current: T, applied: T, original: T) -> Option<T> {
    (current == applied && original != current).then_some(original)
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
    affinity_enabled: bool,
    lower_background_priority: bool,
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
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub requested_game_core_count: Option<i64>,
    pub last_core: bool,
    pub passive: bool,
    pub cpu_mode: topology::CpuMode,
    pub lower_background_priority: bool,
}
fn reorder_enabled(options: Options) -> bool {
    options.cpu_mode != topology::CpuMode::Safe && !options.last_core && !options.passive
}
impl Manager {
    pub fn new(
        config: Value,
        state_path: PathBuf,
        apply_changes: bool,
        restore_on_exit: bool,
        options: Options,
    ) -> Result<Self> {
        let allocation =
            topology::resolve_mode(options.requested_game_core_count, options.cpu_mode)?;
        let affinity_enabled = allocation.supported;
        let game_mask = if options.passive
            || options.cpu_mode == topology::CpuMode::Safe
            || !affinity_enabled
        {
            None
        } else {
            Some(if options.last_core {
                allocation.last_performance_core_mask
            } else {
                allocation.game_mask
            })
        };
        let background_mask = if !affinity_enabled {
            allocation.all_mask
        } else if options.passive {
            allocation.game_mask
        } else if options.last_core {
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
            affinity_enabled,
            lower_background_priority: options.lower_background_priority,
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
            normal_mode: reorder_enabled(options),
        };
        manager.event(
            "helper-created",
            json!({
                "applyChanges": apply_changes,
                "restoreOnExit": restore_on_exit,
                "requestedGameCoreCount": options.requested_game_core_count,
                "gameMask": manager.game_mask.map(|value| format!("0x{value:x}")),
                "backgroundMask": format!("0x{:x}", manager.background_mask),
                "latencyMask": format!("0x{:x}", manager.latency_mask),
                "allocationSource": manager.allocation.source
                ,"cpuMode": format!("{:?}", options.cpu_mode).to_lowercase()
                ,"lowerBackgroundPriority": options.lower_background_priority
                ,"affinitySupported": manager.affinity_enabled
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
        legacy_process_eligible(p,self.current_session)
            && !self.exclude_names.contains(&name)
            && !self.exclude_patterns.iter().any(|r| r.is_match(&name))
            && !process::matches_game(p, &self.config)
    }
    fn latency(&self, p: &Process) -> bool {
        self.eligible(p)
            && (self.latency_names.contains(&p.name.to_lowercase())
                || self.latency_patterns.iter().any(|r| r.is_match(&p.name)))
    }
    fn apply_one(&mut self, p: &Process, target: Option<u64>, role: &str) -> Result<()> {
        let key = key(p);
        if self.handled.contains(&key) {
            return Ok(());
        }
        let result = (|| {
            let priority_target = priority_target(role, self.lower_background_priority);
            if target.is_none() && priority_target.is_none() {
                return Ok(());
            }
            let handle = process::ProcessHandle::open(p.pid, true)?;
            if handle.start_time() != p.start_time {
                return Err("Process identity changed".into());
            }
            let original_mask = handle.affinity()?;
            let original_priority = handle.priority()?;
            let applied_mask = target.unwrap_or(original_mask);
            let applied_priority = priority_target.unwrap_or(original_priority);
            if original_mask == applied_mask && original_priority == applied_priority {
                return Ok(());
            }
            if !self.apply_changes {
                return Ok(());
            }
            self.changed.insert(
                key.clone(),
                Entry {
                    pid: p.pid,
                    start_time: p.start_time.clone(),
                    name: p.name.clone(),
                    original_mask: format!("0x{original_mask:x}"),
                    applied_mask: format!("0x{applied_mask:x}"),
                    role: role.into(),
                    original_priority_class: (original_priority != applied_priority)
                        .then_some(original_priority),
                    applied_priority_class: (original_priority != applied_priority)
                        .then_some(applied_priority),
                },
            );
            self.save()?;
            if original_mask != applied_mask {
                handle.set_affinity(applied_mask)?;
            }
            if original_priority != applied_priority {
                handle.set_priority(applied_priority)?;
            }
            Ok(())
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
                Some((self.game_mask, "game"))
            } else if self.latency(p) {
                Some((
                    self.affinity_enabled.then_some(self.latency_mask),
                    "latency",
                ))
            } else if self.eligible(p) {
                Some((
                    self.affinity_enabled.then_some(self.background_mask),
                    "background",
                ))
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
        let unresolved = self.restore_entries_with_reason(entries, "explicit")?;
        if unresolved.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{}개 프로세스의 원본 상태 복원을 완료하지 못했습니다",
                unresolved.len()
            ))
        }
    }
    fn restore_entries_with_reason(&self, entries: &[Entry], reason: &str) -> Result<Vec<Entry>> {
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
        let (mut missing, mut identity_changed, mut restored, mut preserved, mut failed) =
            (0, 0, 0, 0, 0);
        let mut unresolved = Vec::new();
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
                let handle = match process::ProcessHandle::open(e.pid, true) {
                    Ok(handle) => handle,
                    Err(error) => {
                        failed += 1;
                        unresolved.push(e.clone());
                        eprintln!("[restore-skip] PID={}: {error}", e.pid);
                        continue;
                    }
                };
                if handle.start_time() != e.start_time {
                    identity_changed += 1;
                    continue;
                }
                let result = (|| -> Result<(bool, bool)> {
                    let mut changed = false;
                    let mut externally_changed = false;
                    let current_mask = handle.affinity()?;
                    let applied_mask = mask(&e.applied_mask)?;
                    let original_mask = mask(&e.original_mask)?;
                    if let Some(original) =
                        original_if_unchanged(current_mask, applied_mask, original_mask)
                    {
                        handle.set_affinity(original)?;
                        changed = true;
                    } else if current_mask != applied_mask {
                        externally_changed = true;
                    }
                    if let (Some(original), Some(applied)) =
                        (e.original_priority_class, e.applied_priority_class)
                    {
                        let current = handle.priority()?;
                        if let Some(original) = original_if_unchanged(current, applied, original) {
                            handle.set_priority(original)?;
                            changed = true;
                        } else if current != applied {
                            externally_changed = true;
                        }
                    }
                    Ok((changed, externally_changed))
                })();
                match result {
                    Ok((true, _)) => restored += 1,
                    Ok((false, true)) => preserved += 1,
                    Ok((false, false)) => (),
                    Err(error) => {
                        failed += 1;
                        unresolved.push(e.clone());
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
                "externallyChangedCount": preserved,
                "missingCount": missing,
                "identityChangedCount": identity_changed,
                "failedCount": failed
            }),
        );
        Ok(unresolved)
    }
    pub fn restore_all(&mut self) -> Result<()> {
        self.restore_all_with_reason("explicit-all")
    }
    fn restore_all_with_reason(&mut self, reason: &str) -> Result<()> {
        let unresolved = self.restore_entries_with_reason(&self.entries(), reason)?;
        self.changed = unresolved
            .into_iter()
            .map(|entry| {
                (
                    format!(
                        "{}:{}",
                        entry.pid,
                        entry.start_time.as_deref().unwrap_or("null")
                    ),
                    entry,
                )
            })
            .collect();
        self.handled.clear();
        self.save_or_remove()?;
        if self.changed.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{}개 프로세스의 상태 복원이 남아 있습니다",
                self.changed.len()
            ))
        }
    }
    pub fn stop_without_restore(&mut self) -> Result<()> {
        self.save_or_remove()
    }
    fn save_or_remove(&self) -> Result<()> {
        if self.apply_changes {
            if self.changed.is_empty() {
                match std::fs::remove_file(&self.state_path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                }
            } else {
                self.save()?;
            }
        }
        Ok(())
    }
    pub fn recover(&mut self) -> Result<()> {
        if let Some(v) = storage::read_json(&self.state_path)? {
            let entries: Vec<Entry> =
                serde_json::from_value(v["entries"].clone()).map_err(|e| e.to_string())?;
            let unresolved = self.restore_entries_with_reason(&entries, "startup-recovery")?;
            self.changed = unresolved
                .into_iter()
                .map(|entry| {
                    (
                        format!(
                            "{}:{}",
                            entry.pid,
                            entry.start_time.as_deref().unwrap_or("null")
                        ),
                        entry,
                    )
                })
                .collect();
            self.handled.clear();
            self.save_or_remove()?;
        }
        Ok(())
    }
    pub fn reset_all(&mut self) -> Result<Value> {
        let entry_count = self.changed.len();
        self.restore_all_with_reason("reset-all")?;
        Ok(json!({"restoredEntryCount":entry_count}))
    }
    pub fn legacy_stale_entries(&self, known: &[Entry]) -> Result<Vec<Entry>> {
        let known: HashSet<_> = known.iter().map(entry_key).collect();
        let mut applied = HashSet::from([self.background_mask, self.latency_mask]);
        if let Some(mask) = self.game_mask {
            applied.insert(mask);
        }
        if self.allocation.alternate_game_mask != 0 {
            applied.insert(self.allocation.alternate_game_mask);
        }
        if self.allocation.alternate_background_mask != 0 {
            applied.insert(self.allocation.alternate_background_mask);
        }
        applied.remove(&self.allocation.all_mask);
        let mut entries = Vec::new();
        for process in process::list(true)? {
            if !legacy_process_eligible(&process,self.current_session) {
                continue;
            }
            let Ok(handle) = process::ProcessHandle::open(process.pid, false) else {
                continue;
            };
            if handle.start_time() != process.start_time {
                continue;
            }
            let Ok(current) = handle.affinity() else {
                continue;
            };
            if let Some(entry) = legacy_stale_entry(
                &process,
                current,
                &known,
                &applied,
                self.allocation.all_mask,
            ) {
                entries.push(entry);
            }
        }
        self.event("legacy-stale-affinity-detected",json!({"entryCount":entries.len(),"appliedMasks":applied.iter().map(|mask|format!("0x{mask:x}")).collect::<Vec<_>>(),"fullMask":format!("0x{:x}",self.allocation.all_mask)}));
        Ok(entries)
    }
    fn force(&self, processes: &[&Process], target: u64) -> Value {
        let (mut changed, mut already, mut skipped) = (0, 0, 0);
        for p in processes {
            let result = process::ProcessHandle::open(p.pid, true).and_then(|handle| {
                if handle.start_time() != p.start_time {
                    return Err("Process identity changed".into());
                }
                let current = handle.affinity()?;
                if current == target {
                    return Ok(false);
                }
                if self.apply_changes {
                    handle.set_affinity(target)?;
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
            return Err("CPU 재정렬은 게임 코어 고정을 켠 상태에서만 사용할 수 있습니다".into());
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
        let Ok(handle) = process::ProcessHandle::open(e.pid, false) else {
            return false;
        };
        if e.start_time.is_none() || handle.start_time() != e.start_time {
            return false;
        }
        let affinity_matches = mask(&e.original_mask)
            .ok()
            .zip(mask(&e.applied_mask).ok())
            .zip(handle.affinity().ok())
            .is_some_and(|((original, applied), current)| {
                original != applied && applied == current
            });
        let priority_matches = e
            .applied_priority_class
            .zip(handle.priority().ok())
            .is_some_and(|(applied, current)| applied == current);
        affinity_matches || priority_matches
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

    #[test]
    fn background_priority_is_opt_in_and_excludes_other_roles() {
        assert_eq!(priority_target("background", false), None);
        assert_eq!(
            priority_target("background", true),
            Some(process::BACKGROUND_PRIORITY_CLASS)
        );
        for role in ["game", "latency", "excluded"] {
            assert_eq!(priority_target(role, true), None);
        }
    }

    #[test]
    fn restore_only_replaces_the_value_applied_by_this_app() {
        assert_eq!(original_if_unchanged(5, 5, 9), Some(9));
        assert_eq!(original_if_unchanged(7, 5, 9), None);
        assert_eq!(original_if_unchanged(5, 5, 5), None);
    }

    #[test]
    fn legacy_recovery_only_claims_untracked_exact_app_masks() {
        let process = Process {
            pid: 42,
            name: "example.exe".into(),
            path: Some(r"C:\Apps\example.exe".into()),
            start_time: Some("start".into()),
            session_id: Some(1),
        };
        let applied = HashSet::from([0xfc0c, 0x3f3]);
        let entry =
            legacy_stale_entry(&process, 0xfc0c, &HashSet::new(), &applied, 0xffff).unwrap();
        assert_eq!(entry.original_mask, "0xffff");
        assert_eq!(entry.applied_mask, "0xfc0c");
        assert_eq!(entry.role, "legacy-reset");
        assert!(legacy_stale_entry(&process, 0xaaaa, &HashSet::new(), &applied, 0xffff).is_none());
        assert!(legacy_stale_entry(&process, 0xffff, &HashSet::new(), &applied, 0xffff).is_none());
        assert!(
            legacy_stale_entry(
                &process,
                0xfc0c,
                &HashSet::from([key(&process)]),
                &applied,
                0xffff
            )
            .is_none()
        );
        assert!(legacy_process_eligible(&process,Some(1)));
        assert!(legacy_process_eligible(&Process{path:Some(r"C:\Windows\System32\conhost.exe".into()),..process.clone()},Some(1)));
        assert!(!legacy_process_eligible(&process,Some(2)));
    }

    #[test]
    fn cpu_reorder_requires_explicit_game_binding() {
        assert!(reorder_enabled(Options {
            cpu_mode: topology::CpuMode::Focus,
            ..Default::default()
        }));
        assert!(!reorder_enabled(Options {
            cpu_mode: topology::CpuMode::Safe,
            ..Default::default()
        }));
        assert!(!reorder_enabled(Options {
            cpu_mode: topology::CpuMode::Balanced,
            passive: true,
            ..Default::default()
        }));
    }
}
