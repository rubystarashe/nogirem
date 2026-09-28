use crate::{Result, storage};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Environment {
    pub root: PathBuf,
    pub user: PathBuf,
    pub documents: PathBuf,
    pub videos: PathBuf,
    pub exe: PathBuf,
    pub packaged: bool,
    pub portable: bool,
}
impl Environment {
    pub fn new(root: PathBuf, folders: &Value) -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("USERPROFILE").ok_or("USERPROFILE unavailable")?);
        let portable = root.join("portable.marker").is_file();
        let folder = |key: &str, fallback: &str| {
            folders[key]
                .as_str()
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(fallback))
        };
        Ok(Self {
            root,
            user: std::env::var_os("NOGIREM_USER_DATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::var_os("APPDATA")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| home.join("AppData/Roaming"))
                        .join("마비노기 렘 부스터")
                }),
            documents: folder("documents", "Documents"),
            videos: folder("videos", "Videos"),
            exe: std::env::current_exe().map_err(|e| e.to_string())?,
            packaged: !cfg!(debug_assertions),
            portable,
        })
    }
    pub fn valid_game(&self, path: &str) -> bool {
        let config = read(&self.root.join("config.json"));
        let expected = config["gameExecutableName"]
            .as_str()
            .unwrap_or("Client.exe");
        let path = Path::new(path);
        path.is_file()
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(expected))
    }
    pub fn game(&self) -> String {
        let state = read(&self.user.join("game/path.json"));
        let runtime = read(&self.user.join("affinity/status.json"));
        let config = read(&self.root.join("config.json"));
        for candidate in [&state["executablePath"], &runtime["gameExecutablePath"]] {
            if let Some(path) = candidate.as_str().filter(|p| self.valid_game(p)) {
                return path.into();
            }
        }
        config["gameExecutable"].as_str().unwrap_or("").into()
    }
    pub fn core_count(&self) -> Option<i64> {
        read(&self.user.join("affinity/game-core-setting.json"))["gameCoreCount"].as_i64()
    }
}
pub fn read(path: &Path) -> Value {
    storage::read_json(path)
        .ok()
        .flatten()
        .unwrap_or(Value::Null)
}
pub fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
pub struct ManagedProcess {
    pub directory: PathBuf,
    pub child: Option<Child>,
}
impl ManagedProcess {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            child: None,
        }
    }
    pub fn status(&self) -> Value {
        read(&self.directory.join("status.json"))
    }
    pub fn running(&mut self) -> bool {
        self.child
            .as_mut()
            .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    }
    pub fn control(&self, mut command: Value) -> Result<()> {
        command["requestedAt"] = json!(crate::now_ms());
        storage::write_json(&self.directory.join("control.json"), &command)
    }
    pub fn start(
        &mut self,
        exe: &Path,
        mut args: Vec<String>,
        timeout_ms: u64,
        callback: Option<std::sync::Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Value> {
        if self.running() {
            return Ok(self.status());
        }
        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        remove(&self.directory.join("status.json"))?;
        remove(&self.directory.join("control.json"))?;
        args.extend([
            format!(
                "--status-path={}",
                self.directory.join("status.json").display()
            ),
            format!(
                "--control-path={}",
                self.directory.join("control.json").display()
            ),
            format!("--parent-pid={}", std::process::id()),
        ]);
        use std::os::windows::process::CommandExt;
        let mut child = Command::new(exe)
            .args(args)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(if callback.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let (Some(out), Some(callback)) = (child.stdout.take(), callback) {
            std::thread::spawn(move || {
                for line in BufReader::new(out)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    callback(line)
                }
            });
        }
        if let Some(err) = child.stderr.take() {
            let log = self.directory.join("helper-stderr.log");
            std::thread::spawn(move || {
                crate::helper_log::capture(BufReader::new(err), &log);
            });
        }
        self.child = Some(child);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            let status = self.status();
            if status["running"] == true {
                return Ok(status);
            }
            if !status["error"].is_null() {
                let _ = self.stop();
                return Err(status["error"]
                    .as_str()
                    .or(status["error"]["message"].as_str())
                    .unwrap_or("보조 프로그램 시작 오류")
                    .into());
            }
            if !self.running() {
                return Err("보조 프로그램이 시작 중 종료되었습니다".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.stop();
        Err("보조 프로그램 시작 시간이 초과되었습니다".into())
    }
    pub fn stop(&mut self) -> Result<()> {
        if !self.running() {
            self.child = None;
            return Ok(());
        }
        self.control(json!({"command":"stop"}))?;
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if !self.running() {
                self.child = None;
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50))
        }
        if let Some(mut c) = self.child.take() {
            c.kill().map_err(|e| e.to_string())?;
            c.wait().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
