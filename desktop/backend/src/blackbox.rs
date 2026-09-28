use crate::{
    Result, command, settings, storage, topology,
    workers::{Environment, ManagedProcess, read, remove},
};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub type Event = Arc<dyn Fn(&str, Value) + Send + Sync>;
#[derive(Clone)]
pub struct EditorSession {
    pub id: String,
    pub anchor: u64,
    pub seconds: f64,
    pub timeline: f64,
    pub segments: Vec<Value>,
    pub files: HashMap<String, PathBuf>,
    pub prepared: Option<Value>,
}
pub struct Blackbox {
    pub env: Environment,
    worker: ManagedProcess,
    pub session: Option<EditorSession>,
    event: Event,
    shortcut_available: Arc<AtomicBool>,
    drives: Vec<Value>,
    drives_at: u64,
    summary: Option<Value>,
    last_summary: Value,
    summary_key: String,
    latest: Option<PathBuf>,
    pub approved_clips: Option<PathBuf>,
    last_clip: u64,
    pub saving: bool,
}
fn number(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .filter(|v| v.is_finite())
        .unwrap_or(0.)
}
fn drive(path: &Path) -> String {
    let s = path.to_string_lossy();
    let b = s.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        s[..2].to_uppercase()
    } else {
        "C:".into()
    }
}
fn key(path: &Path) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase()
}
fn ring_arg(paths: &[PathBuf]) -> String {
    format!(
        "--ring-paths={}",
        paths
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join("|")
    )
}
pub fn clip_name(request: &str) -> Result<String> {
    let mut name = request.trim().to_owned();
    if name.to_lowercase().ends_with(".mp4") {
        name.truncate(name.len() - 4)
    }
    if name.is_empty()
        || name.encode_utf16().count() > 120
        || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
        || name.ends_with(['.', ' '])
        || regex::Regex::new(r"(?i)^(con|prn|aux|nul|com[1-9]|lpt[1-9])$")
            .unwrap()
            .is_match(&name)
    {
        return Err("클립 이름에 사용할 수 없는 문자가 있습니다".into());
    }
    Ok(format!("{name}.mp4"))
}
pub fn clip_path(dir: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name.contains(['/', '\\', ':'])
        || !name.to_lowercase().ends_with(".mp4")
        || Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name)
    {
        return Err("허용되지 않은 블랙박스 클립 요청입니다".into());
    }
    Ok(dir.join(name))
}
pub fn shortcut(value: &Value) -> (u32, u32) {
    let accelerator = settings::shortcut(value);
    let parts: Vec<_> = accelerator.split('+').collect();
    let last = parts.last().copied().unwrap_or("");
    let vk = if last.len() == 1 {
        last.as_bytes()[0] as u32
    } else if let Some(f) = last
        .strip_prefix('F')
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| *v >= 1 && *v <= 24)
    {
        0x70 + f - 1
    } else {
        match last {
            "PrintScreen" => 0x2c,
            "Capslock" => 0x14,
            "Numlock" => 0x90,
            "Scrolllock" => 0x91,
            "numadd" => 0x6b,
            "numsub" => 0x6d,
            "nummult" => 0x6a,
            "numdiv" => 0x6f,
            "numdec" => 0x6e,
            "Pause" => 0x13,
            "Space" => 0x20,
            "Tab" => 0x09,
            "Enter" => 0x0d,
            "Escape" => 0x1b,
            "Insert" => 0x2d,
            "Delete" => 0x2e,
            "Home" => 0x24,
            "End" => 0x23,
            "PageUp" => 0x21,
            "PageDown" => 0x22,
            "Up" => 0x26,
            "Down" => 0x28,
            "Left" => 0x25,
            "Right" => 0x27,
            _ => 0,
        }
    };
    let mut modifiers = 0;
    for part in parts {
        modifiers |= match part {
            "CommandOrControl" | "Control" => 2,
            "Alt" => 1,
            "Shift" => 4,
            "Super" => 8,
            _ => 0,
        }
    }
    (vk, modifiers)
}
pub fn pieces(segments: &[Value], start: f64, duration: f64, skip: bool) -> Vec<Value> {
    let end = start + duration;
    let mut cursor = start;
    let mut pieces = vec![];
    for s in segments {
        let t = number(&s["timelineStartSeconds"]).max(0.);
        let m = number(&s["mediaStartSeconds"]).max(0.);
        let duration = number(&s["durationSeconds"]).max(0.);
        if t + duration <= cursor || t >= end {
            continue;
        }
        let a = cursor.max(t);
        let b = end.min(t + duration);
        if a > cursor && !skip {
            pieces.push(json!({"type":"black","duration":a-cursor}))
        }
        pieces.push(json!({"type":"media","start":m+a-t,"duration":b-a}));
        cursor = b
    }
    if cursor < end && !skip {
        pieces.push(json!({"type":"black","duration":end-cursor}))
    }
    pieces
        .into_iter()
        .filter(|p| number(&p["duration"]) >= 0.01)
        .collect()
}
impl Blackbox {
    pub fn new(env: Environment, event: Event) -> Self {
        Self {
            worker: ManagedProcess::new(env.user.join("blackbox")),
            env,
            event,
            session: None,
            shortcut_available: Arc::new(AtomicBool::new(true)),
            drives: vec![],
            drives_at: 0,
            summary: None,
            last_summary: Value::Null,
            summary_key: String::new(),
            latest: None,
            approved_clips: None,
            last_clip: 0,
            saving: false,
        }
    }
    fn base(&self) -> PathBuf {
        self.env.videos.join("마비노기 렘 블랙박스")
    }
    fn saved(&self) -> Value {
        settings::blackbox(&read(&self.worker.directory.join("settings.json")))
    }
    pub fn clips(&self) -> PathBuf {
        let s = self.saved();
        let p = PathBuf::from(s["clipStoragePath"].as_str().unwrap_or(""));
        if p.is_absolute() {
            p
        } else {
            self.base().join("Clips")
        }
    }
    fn active_ring(&self, s: &Value) -> PathBuf {
        let base = self.base();
        let wanted = s["ringStorageDrive"].as_str().unwrap_or("");
        if wanted.is_empty() || wanted.eq_ignore_ascii_case(&drive(&base)) {
            base.join("Ring")
        } else {
            PathBuf::from(format!("{wanted}\\마비노기 렘 블랙박스\\Ring"))
        }
    }
    fn affinity(&self) -> Result<String> {
        let a = topology::resolve(self.env.core_count())?;
        let isolated = a.background_mask & !a.alternate_game_mask;
        let mask = if isolated != 0 {
            isolated
        } else {
            a.background_mask
        };
        Ok(format!("--affinity-mask=0x{mask:x}"))
    }
    pub fn utility(
        &self,
        mut args: Vec<String>,
        progress: Option<Arc<dyn Fn(u32) + Send + Sync>>,
    ) -> Result<String> {
        args.push(self.affinity()?);
        let output = command::run_progress(
            self.env
                .root
                .join("native/recorder-helper/tools-bin/recorder-helper.exe"),
            &args,
            7200000,
            1024 * 1024,
            progress,
        )?;
        if !output.success {
            return Err(format!(
                "블랙박스 편집 작업 실패 ({:?}): {}",
                output.code,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }
    pub fn storage_drives(&mut self) -> Vec<Value> {
        if !self.drives.is_empty() && crate::now_ms().saturating_sub(self.drives_at) < 300000 {
            return self.drives.clone();
        }
        let parsed = self
            .utility(vec!["--mode=drives".into()], None)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null);
        self.drives = parsed
            .as_array()
            .cloned()
            .unwrap_or_else(|| vec![parsed])
            .iter()
            .filter_map(|v| {
                let id = v["id"].as_str()?.to_uppercase();
                let b = id.as_bytes();
                (b.len() == 2 && b[0].is_ascii_uppercase() && b[1] == b':')
                    .then(|| json!({"id":id,"freeBytes":number(&v["freeBytes"]).max(0.) as u64}))
            })
            .collect();
        self.drives
            .sort_by_key(|d| d["id"].as_str().unwrap().to_owned());
        if self.drives.is_empty() {
            self.drives
                .push(json!({"id":drive(&self.base()),"freeBytes":0}))
        }
        self.drives_at = crate::now_ms();
        self.drives.clone()
    }
    pub fn rings(&mut self) -> Vec<PathBuf> {
        let mut paths = vec![self.active_ring(&self.saved()), self.base().join("Ring")];
        for d in self.storage_drives() {
            paths.push(PathBuf::from(format!(
                "{}\\마비노기 렘 블랙박스\\Ring",
                d["id"].as_str().unwrap()
            )))
        }
        let mut seen = HashSet::new();
        paths.into_iter().filter(|p| seen.insert(key(p))).collect()
    }
    pub fn status(&mut self) -> Result<Value> {
        let mut setting = self.saved();
        let status = self.worker.status();
        let paths = self.rings();
        let summary_key = paths.iter().map(|p| key(p)).collect::<Vec<_>>().join("|");
        let process = self.worker.running();
        let fresh =
            crate::now_ms().saturating_sub(status["updatedAt"].as_u64().unwrap_or(0)) < 5000;
        let running = setting["enabled"] == true && process && fresh && status["running"] == true;
        if !process && (self.summary.is_none() || self.summary_key != summary_key) {
            if let Ok(out) = self.utility(vec!["--mode=summary".into(), ring_arg(&paths)], None) {
                if let Ok(summary) = serde_json::from_str::<Value>(&out) {
                    self.summary = Some(summary);
                    self.summary_key = summary_key.clone();
                }
            }
        }
        let summary = if process && fresh {
            status.clone()
        } else if !process && self.summary_key == summary_key {
            self.summary.clone().unwrap_or(self.last_summary.clone())
        } else {
            self.last_summary.clone()
        };
        self.last_summary = summary.clone();
        let quality = settings::quality(
            &setting,
            std::thread::available_parallelism().map_or(4, |n| n.get() as u64),
            crate::memory::snapshot().map_or(0, |s| s.total),
        );
        let mut runtime = setting.clone();
        runtime["quality"] = json!(quality);
        let active = self.active_ring(&setting);
        let shortcut = setting["shortcut"].as_str().unwrap_or("").to_owned();
        for field in ["bytesUsed", "durationSeconds"] {
            setting[field] = json!(number(&summary[field]).max(0.))
        }
        for field in ["audioGainDb", "droppedFrames", "width", "height"] {
            setting[field] = json!(number(&status[field]))
        }
        for field in [
            "recording",
            "audioRecording",
            "waitingForGame",
            "clipInProgress",
        ] {
            setting[field] = json!(running && status[field] == true)
        }
        setting["running"] = json!(running);
        setting["audioError"] = if fresh {
            status["audioError"].clone()
        } else {
            Value::Null
        };
        setting["runtimeUpdatedAt"] = json!(number(&status["updatedAt"]).max(0.));
        setting["capacityBytes"] =
            json!(setting["capacityGb"].as_u64().unwrap() * 1024 * 1024 * 1024);
        setting["resolvedQuality"] = json!(quality);
        setting["bitrateMbps"] = json!(settings::bitrate(&runtime));
        setting["storagePath"] = json!(self.base().to_string_lossy());
        setting["storageDrives"] = json!(self.drives);
        setting["ringStorageDrive"] = json!(drive(&active));
        setting["ringStoragePath"] = json!(active.to_string_lossy());
        setting["ringStoragePaths"] = json!(paths);
        setting["clipStoragePath"] = json!(self.clips().to_string_lossy());
        if self.latest.as_ref().is_some_and(|p| !p.exists()) {
            self.latest = None
        }
        setting["latestClip"] = self
            .latest
            .as_ref()
            .map(|p| json!(p.to_string_lossy()))
            .unwrap_or(status["latestClip"].clone());
        setting["shortcutAccelerator"] = json!(shortcut);
        setting["shortcut"] = json!(if shortcut.is_empty() {
            "사용 안 함".into()
        } else {
            shortcut
                .replace("CommandOrControl", "Ctrl")
                .replace("Control", "Ctrl")
                .replace('+', " + ")
        });
        setting["shortcutAvailable"] =
            json!(shortcut.is_empty() || self.shortcut_available.load(Ordering::Relaxed));
        setting["reason"] = if fresh {
            status["error"].clone()
        } else if setting["enabled"] == true && !running {
            json!("블랙박스 녹화 프로세스가 실행 중이 아닙니다")
        } else {
            Value::Null
        };
        Ok(setting)
    }
    pub fn start(&mut self) -> Result<Value> {
        if self.worker.running() {
            return self.status();
        }
        let s = self.saved();
        let paths = self.rings();
        let quality = settings::quality(
            &s,
            std::thread::available_parallelism().map_or(4, |n| n.get() as u64),
            crate::memory::snapshot()?.total,
        );
        let mut runtime = s.clone();
        runtime["quality"] = json!(quality);
        let (vk, modifiers) = shortcut(&s["shortcut"]);
        let args = vec![
            format!(
                "--metrics-path={}",
                self.worker.directory.join("recorder-metrics.log").display()
            ),
            format!("--storage-path={}", self.base().display()),
            format!("--ring-path={}", self.active_ring(&s).display()),
            ring_arg(&paths),
            format!("--clips-path={}", self.clips().display()),
            format!("--game-path={}", self.env.game()),
            format!("--codec={}", s["codec"].as_str().unwrap()),
            format!("--fps={}", s["fps"]),
            format!("--bitrate-mbps={}", settings::bitrate(&runtime)),
            format!(
                "--max-height={}",
                match quality.as_str() {
                    "1080p" => 1080,
                    "original" => 0,
                    _ => 1440,
                }
            ),
            format!("--chunk-seconds={}", s["chunkSeconds"]),
            format!("--capacity-gb={}", s["capacityGb"]),
            format!("--max-duration-seconds={}", s["maxDurationSeconds"]),
            format!("--shortcut-vk={vk}"),
            format!("--shortcut-modifiers={modifiers}"),
            self.affinity()?,
        ];
        let available = self.shortcut_available.clone();
        let event = self.event.clone();
        self.summary = None;
        self.worker.start(
            &self
                .env
                .root
                .join("native/recorder-helper/bin/recorder-helper.exe"),
            args,
            15000,
            Some(Arc::new(move |line| match line.as_str() {
                "SHORTCUT" => event("blackbox-shortcut", Value::Null),
                "SHORTCUT_READY" => available.store(true, Ordering::Relaxed),
                "SHORTCUT_UNAVAILABLE" => available.store(false, Ordering::Relaxed),
                _ => {}
            })),
        )?;
        self.status()
    }
    pub fn flush_for_update(&mut self) -> Result<()> {
        if self.status()?["recording"] == true {
            let id=crate::now_ms();
            self.worker.control(json!({"command":"flush","requestId":id}))?;
            self.wait_status(10000,|s|s["flushCompletedId"]==id).ok_or("녹화 저장 완료를 확인하지 못해 업데이트 설치를 중단했습니다")?;
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        let _ = self.status();
        self.worker.stop()?;
        self.summary = None;
        Ok(())
    }
    pub fn set(&mut self, patch: &Value) -> Result<Value> {
        let old = self.saved();
        let mut merged = old.clone();
        if let Some(fields) = patch.as_object() {
            for (k, v) in fields {
                merged[k] = v.clone()
            }
        } else {
            return Err("블랙박스 설정 값이 올바르지 않습니다".into());
        }
        let next = settings::blackbox(&merged);
        let next_ring = self.active_ring(&next);
        if next_ring != self.active_ring(&old)
            && !self
                .storage_drives()
                .iter()
                .any(|d| d["id"] == next["ringStorageDrive"])
        {
            return Err("사용할 수 없는 녹화 청크 저장 드라이브입니다".into());
        }
        let next_clip = PathBuf::from(next["clipStoragePath"].as_str().unwrap_or(""));
        let next_clip = if next_clip.is_absolute() {
            next_clip
        } else {
            self.base().join("Clips")
        };
        if next_clip != self.clips() && self.approved_clips.as_ref() != Some(&next_clip) {
            return Err("클립 저장 위치를 다시 선택해 주세요".into());
        }
        let restart = [
            "featureEnabled",
            "enabled",
            "codec",
            "quality",
            "capacityGb",
            "maxDurationSeconds",
            "ringStorageDrive",
            "clipStoragePath",
            "fps",
            "chunkSeconds",
        ]
        .iter()
        .any(|k| old[*k] != next[*k]);
        let running = self.worker.running();
        if restart {
            self.stop()?
        }
        let mut persisted = next.clone();
        persisted["updatedAt"] = json!(crate::now_ms());
        storage::write_json(&self.worker.directory.join("settings.json"), &persisted)?;
        self.approved_clips = None;
        if old["clipStoragePath"] != next["clipStoragePath"] {
            self.latest = None
        }
        if !restart && running && old["shortcut"] != next["shortcut"] {
            let (vk, modifiers) = shortcut(&next["shortcut"]);
            self.worker.control(
                json!({"command":"shortcut","shortcutVirtualKey":vk,"shortcutModifiers":modifiers}),
            )?;
            std::thread::sleep(Duration::from_millis(150));
        }
        if next["enabled"] == true && (restart || !running) {
            if let Err(e) = self.start() {
                persisted["enabled"] = json!(false);
                storage::write_json(&self.worker.directory.join("settings.json"), &persisted)?;
                return Err(e);
            }
        }
        self.status()
    }
    pub fn enabled(&mut self, enabled: bool) -> Result<Value> {
        if enabled && self.saved()["featureEnabled"] != true {
            return Err("고급 기능에서 블랙박스 기능을 먼저 사용 설정해 주세요".into());
        }
        self.set(&json!({"enabled":enabled}))
    }
    pub fn list_clips(&self) -> Result<Value> {
        let dir = self.clips();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut clips = vec![];
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().map_err(|e| e.to_string())?.is_file()
                || !name.to_lowercase().ends_with(".mp4")
            {
                continue;
            }
            let info = entry.metadata().map_err(|e| e.to_string())?;
            let time = info
                .modified()
                .map_err(|e| e.to_string())?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            clips.push(json!({"name":name,"size":info.len(),"modifiedAt":time,"videoUrl":format!("http://nogirem-blackbox.clips/{}?v={time}",urlencoding::encode(&name))}));
        }
        clips.sort_by(|a, b| number(&b["modifiedAt"]).total_cmp(&number(&a["modifiedAt"])));
        Ok(json!(clips))
    }
    pub fn suggest_name(&self) -> Result<String> {
        let list = self.list_clips()?;
        let re = regex::Regex::new(r"(?i)^(\d+)번째 클립\.mp4$").unwrap();
        let high = list
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| {
                re.captures(v["name"].as_str().unwrap())
                    .and_then(|c| c[1].parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0);
        Ok(format!("{}번째 클립", high + 1))
    }
    pub fn rename_clip(&mut self, old: &str, new: &str) -> Result<Value> {
        let source = clip_path(&self.clips(), old)?;
        let name = clip_name(new)?;
        let target = clip_path(&self.clips(), &name)?;
        if source == target {
            return Ok(json!({"name":name}));
        }
        if key(&source) == key(&target) {
            let temp = source.with_extension(format!("{}.rename", uuid::Uuid::new_v4()));
            fs::rename(&source, &temp).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&temp, &target) {
                let _ = fs::rename(&temp, &source);
                return Err(e.to_string());
            }
        } else {
            if target.exists() {
                return Err("같은 이름의 클립이 이미 있습니다".into());
            }
            fs::rename(source, &target).map_err(|e| e.to_string())?
        }
        Ok(json!({"name":name}))
    }
    pub fn delete_clip(&self, name: &str) -> Result<Value> {
        let path = clip_path(&self.clips(), name)?;
        for attempt in 0..5 {
            match fs::remove_file(&path) {
                Ok(()) => return Ok(json!({"deleted":true})),
                Err(e) if attempt < 4 && matches!(e.raw_os_error(), Some(5 | 32 | 33)) => {
                    std::thread::sleep(Duration::from_millis(100))
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        unreachable!()
    }
    fn wait_status(&self, timeout: u64, mut test: impl FnMut(&Value) -> bool) -> Option<Value> {
        let deadline = Instant::now() + Duration::from_millis(timeout);
        while Instant::now() < deadline {
            let s = self.worker.status();
            if test(&s) {
                return Some(s);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }
    pub fn quick_clip(&mut self, requested_name: &str) -> Result<Value> {
        if self.saving {
            return Err("이전 클립을 저장하고 있습니다".into());
        }
        if crate::now_ms().saturating_sub(self.last_clip) < 2000 {
            return Err("이전 클립 요청을 처리하고 있습니다".into());
        }
        let status = self.status()?;
        if status["running"] != true {
            return Err("블랙박스 녹화가 실행 중이 아닙니다".into());
        }
        if status["recording"] != true {
            return Err("마비노기 플레이 중에만 클립을 저장할 수 있습니다".into());
        }
        if status["clipInProgress"] == true {
            return Err("이전 클립을 저장하고 있습니다".into());
        }
        self.saving = true;
        (self.event)(
            "blackbox-notification",
            json!({"message":"클립을 저장 중입니다","persistent":true}),
        );
        let result = (|| {
            let name = clip_name(&if requested_name.trim().is_empty() {
                self.suggest_name()?
            } else {
                requested_name.trim().into()
            })?;
            if clip_path(&self.clips(), &name)?.exists() {
                return Err("같은 이름의 클립이 이미 있습니다".into());
            }
            let previous = self.worker.status();
            self.worker
                .control(json!({"command":"clip","seconds":status["clipSeconds"]}))?;
            self.last_clip = crate::now_ms();
            let mut clip_started = false;
            let completed = self
                .wait_status(120000, |v| {
                    if v["clipInProgress"] == true {
                        clip_started = true;
                    }
                    v["clipInProgress"] != true
                        && ((!v["error"].is_null()
                            && (clip_started || v["error"] != previous["error"]))
                            || (!v["latestClip"].is_null()
                                && v["latestClip"] != previous["latestClip"]))
                })
                .ok_or("클립 저장이 제한 시간 안에 완료되지 않았습니다")?;
            if let Some(e) = completed["error"].as_str() {
                return Err(e.into());
            }
            let output = PathBuf::from(
                completed["latestClip"]
                    .as_str()
                    .ok_or("저장된 클립 경로가 없습니다")?,
            );
            if output.parent().map(key) != Some(key(&self.clips())) {
                return Err("허용되지 않은 클립 출력 경로".into());
            }
            self.rename_clip(
                output
                    .file_name()
                    .unwrap()
                    .to_str()
                    .ok_or("클립 이름 오류")?,
                &name,
            )?;
            self.latest = Some(self.clips().join(name));
            self.status()
        })();
        self.saving = false;
        (self.event)(
            "blackbox-notification",
            json!({"message":if result.is_ok(){format!("{}초 클립이 저장되었습니다",status["clipSeconds"])}else{"클립 저장에 실패했습니다".into()},"persistent":false}),
        );
        result
    }
    pub fn clear(&mut self) -> Result<Value> {
        let status = self.status()?;
        let rings = self.rings();
        let active = self.active_ring(&self.saved());
        if status["running"] == true {
            let id = crate::now_ms();
            self.worker
                .control(json!({"command":"clear","requestId":id}))?;
            let s = self
                .wait_status(60000, |s| s["clearCompletedId"] == id)
                .ok_or("순환 녹화를 제한 시간 안에 비우지 못했습니다")?;
            if number(&s["bytesUsed"]) > 0. {
                return Err("사용 중인 녹화 청크가 있어 일부 파일을 정리하지 못했습니다".into());
            }
        }
        for ring in rings {
            if status["running"] == true && key(&ring) == key(&active) {
                continue;
            }
            clear_ring(&ring)?;
        }
        if status["running"] != true {
            fs::create_dir_all(&active).map_err(|e| e.to_string())?;
            remove(&self.worker.directory.join("status.json"))?;
        }
        self.summary = None;
        self.summary_key.clear();
        let mut result = self.status()?;
        result["cleared"] = json!(true);
        Ok(result)
    }
    pub fn prepare(&mut self) -> Result<Value> {
        if let Some(s) = &self.session {
            if let Some(v) = &s.prepared {
                return Ok(v.clone());
            }
        }
        let state = self.status()?;
        if state["recording"] == true {
            let id = crate::now_ms();
            self.worker
                .control(json!({"command":"flush","requestId":id}))?;
            let _ = self.wait_status(2000, |s| s["flushCompletedId"] == id);
        }
        let rings = self.rings();
        let anchor = self
            .utility(vec!["--mode=latest".into(), ring_arg(&rings)], None)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v["latestEndMilliseconds"].as_u64())
            .filter(|v| *v > 0)
            .unwrap_or_else(crate::now_ms);
        self.session = Some(EditorSession {
            id: uuid::Uuid::new_v4().to_string(),
            anchor,
            seconds: 900.,
            timeline: 900.,
            segments: vec![],
            files: HashMap::new(),
            prepared: None,
        });
        let result = self.track(&json!(900))?;
        self.session.as_mut().unwrap().prepared = Some(result.clone());
        Ok(result)
    }
    pub fn track(&mut self, requested: &Value) -> Result<Value> {
        let s = self
            .session
            .as_ref()
            .ok_or("블랙박스 편집 세션이 없습니다")?;
        let all = requested == "all";
        let n = number(requested);
        let seconds = if n == 0. {
            900.
        } else {
            n.round().clamp(30., 2147483647.)
        };
        let anchor = s.anchor;
        let id = s.id.clone();
        let paths = self.rings();
        let mut args = vec![
            "--mode=index".into(),
            ring_arg(&paths),
            format!("--anchor-ms={anchor}"),
            format!("--seconds={seconds}"),
        ];
        if all {
            args.push("--all=true".into())
        }
        let metadata: Value =
            serde_json::from_str(&self.utility(args, None)?).map_err(|e| e.to_string())?;
        let timeline = number(&metadata["timelineDurationSeconds"]);
        let timeline = if timeline > 0. { timeline } else { seconds };
        let allowed: HashSet<_> = paths.iter().map(|p| key(p)).collect();
        let mut files = HashMap::new();
        let mut segments = vec![];
        for chunk in metadata["chunks"].as_array().into_iter().flatten() {
            let name = chunk["fileName"].as_str().unwrap_or("");
            if !regex::Regex::new(r"^chunk-\d+\.mp4$")
                .unwrap()
                .is_match(name)
            {
                continue;
            }
            let path = PathBuf::from(chunk["filePath"].as_str().unwrap_or(""));
            if !path.is_file()
                || path.file_name().and_then(|n| n.to_str()) != Some(name)
                || !path.parent().is_some_and(|p| allowed.contains(&key(p)))
            {
                continue;
            }
            let track = uuid::Uuid::new_v4().to_string();
            files.insert(track.clone(), path);
            segments.push(json!({"timelineStartSeconds":number(&chunk["timelineStartSeconds"]),"mediaStartSeconds":number(&chunk["mediaStartSeconds"]),"durationSeconds":number(&chunk["durationSeconds"]),"videoUrl":format!("http://nogirem-blackbox.editor/{id}/{track}?v={name}")}));
        }
        let result = json!({"anchorAt":anchor,"gaps":metadata["gaps"].as_array().cloned().unwrap_or_default(),"videoUrl":segments.first().and_then(|s|s["videoUrl"].as_str()).unwrap_or(""),"segments":segments,"timelineDurationSeconds":timeline,"requestedSeconds":timeline});
        let s = self.session.as_mut().unwrap();
        s.seconds = timeline;
        s.timeline = timeline;
        s.segments = metadata["segments"].as_array().cloned().unwrap_or_default();
        s.files = files;
        Ok(result)
    }
    pub fn extract(
        &mut self,
        v: &Value,
        progress: Arc<dyn Fn(u32) + Send + Sync>,
    ) -> Result<Value> {
        if self.saving {
            return Err("이전 클립을 저장하고 있습니다".into());
        }
        let s = self
            .session
            .as_ref()
            .ok_or("먼저 편집 트랙을 준비하세요")?
            .clone();
        let start = number(&v["startSeconds"]).max(0.);
        let duration = number(&v["durationSeconds"]);
        let duration = if duration == 0. {
            30.
        } else {
            duration.clamp(1., 21600.)
        };
        if start + duration > s.timeline + 0.5 {
            return Err("선택한 추출 구간이 편집 트랙을 벗어났습니다".into());
        }
        let name = v["requestedName"].as_str().unwrap_or("").trim();
        let name = if name.is_empty() {
            format!(
                "마비노기-추출-{}.mp4",
                chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
            )
        } else {
            clip_name(name)?
        };
        let output = clip_path(&self.clips(), &name)?;
        if output.exists() {
            return Err("같은 이름의 클립이 이미 있습니다".into());
        }
        let pieces = pieces(&s.segments, start, duration, v["gapPolicy"] == "skip");
        if pieces.is_empty() {
            return Err("건너뛰기 후 추출할 녹화 영상이 없습니다".into());
        }
        let encoded = pieces
            .iter()
            .map(|p| {
                if p["type"] == "media" {
                    format!(
                        "media:{}:{}",
                        (number(&p["start"]) * 1000.).round(),
                        (number(&p["duration"]) * 1000.).round()
                    )
                } else {
                    format!("black:{}", (number(&p["duration"]) * 1000.).round())
                }
            })
            .collect::<Vec<_>>()
            .join(";");
        let speed = number(&v["playbackSpeed"]);
        let speed = if [0.5, 0.75, 1., 1.25, 1.5, 2.].contains(&speed) {
            speed
        } else {
            1.
        };
        self.saving = true;
        progress(1);
        let rings = self.rings();
        let result = self.utility(
            vec![
                "--mode=compose".into(),
                ring_arg(&rings),
                format!("--anchor-ms={}", s.anchor),
                format!("--seconds={}", s.seconds),
                format!("--output={}", output.display()),
                format!("--pieces={encoded}"),
                format!("--speed-milli={}", (speed * 1000.).round()),
            ],
            Some(progress),
        );
        self.saving = false;
        result?;
        Ok(json!({"outputPath":output.to_string_lossy(),"fileName":name}))
    }
    pub fn protocol_path(&self, url: &str) -> Result<PathBuf> {
        let url = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
        let host = url.host_str().unwrap_or("");
        if host == "clips" || host == "nogirem-blackbox.clips" {
            return clip_path(
                &self.clips(),
                &urlencoding::decode(url.path().trim_start_matches('/'))
                    .map_err(|e| e.to_string())?,
            );
        }
        let parts: Vec<_> = url.path().split('/').filter(|s| !s.is_empty()).collect();
        if !(host == "editor" || host == "nogirem-blackbox.editor") || parts.len() != 2 {
            return Err("허용되지 않은 블랙박스 영상 요청입니다".into());
        }
        let s = self
            .session
            .as_ref()
            .filter(|s| s.id == parts[0])
            .ok_or("허용되지 않은 블랙박스 세션")?;
        s.files
            .get(parts[1])
            .cloned()
            .ok_or("허용되지 않은 블랙박스 영상 요청입니다".into())
    }
}
fn clear_ring(path: &Path) -> Result<()> {
    if path.file_name().and_then(|s| s.to_str()) != Some("Ring")
        || path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            != Some("마비노기 렘 블랙박스")
    {
        return Err("허용되지 않은 녹화 청크 디렉터리".into());
    }
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    use std::os::windows::fs::MetadataExt;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err("재분석 지점의 녹화 청크는 정리할 수 없습니다".into());
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let chunk = regex::Regex::new(r"^chunk-\d+(?:\.partial)?\.mp4$").unwrap();
    for e in fs::read_dir(&canonical).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_file()
            && chunk.is_match(&e.file_name().to_string_lossy())
        {
            let p = e.path();
            if p.parent() != Some(canonical.as_path()) {
                return Err("청크 경로 범위 오류".into());
            }
            fs::remove_file(p).map_err(|e| e.to_string())?
        }
    }
    Ok(())
}
pub fn video_response(path: &Path, method: &str, range: Option<&str>) -> Result<Value> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let invalid =
        || json!({"status":416,"headers":{"Content-Range":format!("bytes */{size}")},"body":""});
    if size == 0 {
        return Ok(
            json!({"status":200,"headers":{"Content-Length":"0","Content-Type":"video/mp4"},"body":""}),
        );
    }
    let (mut start, mut end, mut status) = (0, size - 1, 200);
    if let Some(range) = range {
        let re = regex::Regex::new(r"(?i)^bytes=(\d*)-(\d*)$").unwrap();
        let Some(c) = re.captures(range.trim()) else {
            return Ok(invalid());
        };
        if c[1].is_empty() {
            let Some(n) = c[2].parse::<u64>().ok().filter(|n| *n > 0) else {
                return Ok(invalid());
            };
            start = size.saturating_sub(n)
        } else {
            let Ok(n) = c[1].parse::<u64>() else {
                return Ok(invalid());
            };
            start = n;
            if !c[2].is_empty() {
                let Ok(n) = c[2].parse::<u64>() else {
                    return Ok(invalid());
                };
                end = n
            }
        }
        if end < start || start >= size {
            return Ok(invalid());
        }
        end = end.min(size - 1);
        status = 206
    }
    if method != "HEAD" && end - start + 1 > 4 * 1024 * 1024 {
        end = start + 4 * 1024 * 1024 - 1;
        status = 206
    }
    let mut headers = json!({"Accept-Ranges":"bytes","Content-Type":"video/mp4","Content-Length":(end-start+1).to_string(),"Access-Control-Allow-Origin":"http://dioxus.index.html"});
    if status == 206 {
        headers["Content-Range"] = json!(format!("bytes {start}-{end}/{size}"))
    }
    let mut bytes = vec![];
    if method != "HEAD" {
        file.seek(SeekFrom::Start(start))
            .map_err(|e| e.to_string())?;
        file.take(end - start + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
    }
    Ok(
        json!({"status":status,"headers":headers,"body":base64::engine::general_purpose::STANDARD.encode(bytes)}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gap_extraction() {
        let segments =
            vec![json!({"timelineStartSeconds":5,"mediaStartSeconds":0,"durationSeconds":10})];
        assert_eq!(
            pieces(&segments, 0., 20., false),
            vec![
                json!({"type":"black","duration":5.0}),
                json!({"type":"media","start":0.0,"duration":10.0}),
                json!({"type":"black","duration":5.0})
            ]
        );
        assert_eq!(pieces(&segments, 0., 20., true).len(), 1);
    }
    #[test]
    fn clip_names_reject_traversal() {
        for s in ["../clip", "CON", "bad/name", "clip.", ""] {
            assert!(clip_name(s).is_err())
        }
        assert_eq!(clip_name("test.MP4").unwrap(), "test.mp4");
    }
    #[test]
    fn cleanup_only_removes_owned_chunks_and_preserves_saved_clips() {
        let root =
            std::env::temp_dir().join(format!("nogirem-cleanup-test-{}", uuid::Uuid::new_v4()));
        let ring = root.join("마비노기 렘 블랙박스/Ring");
        fs::create_dir_all(&ring).unwrap();
        for file in [
            "chunk-123.mp4",
            "chunk-456.partial.mp4",
            "saved.mp4",
            "notes.txt",
            "chunk-other.mp4",
        ] {
            fs::write(ring.join(file), b"fixture").unwrap();
        }
        let clips = ring.parent().unwrap().join("Clips");
        fs::create_dir(&clips).unwrap();
        fs::write(clips.join("saved.mp4"), b"keep").unwrap();
        assert!(clear_ring(&root).is_err());
        clear_ring(&ring).unwrap();
        assert!(!ring.join("chunk-123.mp4").exists());
        assert!(!ring.join("chunk-456.partial.mp4").exists());
        for file in ["saved.mp4", "notes.txt", "chunk-other.mp4"] {
            assert!(ring.join(file).exists());
        }
        assert_eq!(fs::read(clips.join("saved.mp4")).unwrap(), b"keep");
        clear_ring(&ring).unwrap();
        // This uniquely created temporary directory contains test fixtures only.
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn media_range_head_suffix_and_limits() {
        let path =
            std::env::temp_dir().join(format!("nogirem-range-test-{}.mp4", uuid::Uuid::new_v4()));
        fs::write(&path, (0..100u8).collect::<Vec<_>>()).unwrap();
        for (range, expected) in [
            ("bytes=10-19", (10..20u8).collect::<Vec<_>>()),
            ("bytes=-7", (93..100u8).collect()),
            ("bytes=97-200", (97..100u8).collect()),
        ] {
            let response = video_response(&path, "GET", Some(range)).unwrap();
            assert_eq!(response["status"], 206);
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(response["body"].as_str().unwrap())
                    .unwrap(),
                expected
            );
        }
        for range in [
            "bytes=100-",
            "bytes=20-10",
            "bytes=-0",
            "bytes=0-1,3-4",
            "bogus",
        ] {
            assert_eq!(
                video_response(&path, "GET", Some(range)).unwrap()["status"],
                416
            );
        }
        let head = video_response(&path, "HEAD", None).unwrap();
        assert_eq!(head["body"], "");
        assert_eq!(head["headers"]["Content-Length"], "100");
        fs::File::create(&path)
            .unwrap()
            .set_len(8 * 1024 * 1024)
            .unwrap();
        let capped = video_response(&path, "GET", None).unwrap();
        assert_eq!(capped["status"], 206);
        assert_eq!(capped["headers"]["Content-Length"], "4194304");
        fs::remove_file(path).unwrap();
    }
}
