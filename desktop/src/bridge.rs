use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

type Reply = Result<Value, String>;
struct Inner {
    output: std::sync::mpsc::Sender<Value>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Reply>>>,
    sequence: AtomicU64,
    child: Mutex<Child>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[derive(Clone)]
pub struct Client(Arc<Inner>);

impl Client {
    pub fn start(
        root: &Path,
        displays: Value,
    ) -> Result<(Self, mpsc::UnboundedReceiver<Value>), String> {
        let executable = std::env::current_exe().map_err(|e|e.to_string())?;
        let mut command = Command::new(executable);
        command
            .arg("--native-service")
            .env("NOGIREM_ROOT",root)
            .env("NOGIREM_DESKTOP_PID",std::process::id().to_string())
            .current_dir(root)
            .env("NOGIREM_PACKAGED", if cfg!(debug_assertions) { "0" } else { "1" })
            .env(
                "NOGIREM_DESKTOP_EXE",
                std::env::current_exe().map_err(|e| e.to_string())?,
            )
            .env("NOGIREM_DISPLAYS", displays.to_string())
            .env(
                "NOGIREM_KNOWN_FOLDERS",
                crate::platform::known_folders().to_string(),
            )
            .args(std::env::args().skip(1))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("서비스를 시작할 수 없습니다: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("Missing service stdin")?;
        let stdout = child.stdout.take().ok_or("Missing service stdout")?;
        let (output, writes) = std::sync::mpsc::channel::<Value>();
        let (incoming, receiver) = mpsc::unbounded_channel();
        let inner = Arc::new(Inner {
            output,
            pending: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(1),
            child: Mutex::new(child),
        });
        std::thread::spawn(move || {
            for message in writes {
                if writeln!(stdin, "{message}")
                    .and_then(|_| stdin.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let weak = Arc::downgrade(&inner);
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if message["type"] == "response" {
                    if let Some(inner) = weak.upgrade() {
                        if let Some(id) = message["id"].as_u64() {
                            if let Some(reply) = inner.pending.lock().unwrap().remove(&id) {
                                let result = match message["error"].as_str() {
                                    Some(error) => Err(error.into()),
                                    None => Ok(message["result"].clone()),
                                };
                                let _ = reply.send(result);
                            }
                        }
                    }
                } else if incoming.send(message).is_err() {
                    break;
                }
            }
            if let Some(inner) = weak.upgrade() {
                for (_, reply) in inner.pending.lock().unwrap().drain() {
                    let _ = reply.send(Err("서비스 연결이 종료되었습니다".into()));
                }
            }
            let _ = incoming.send(json!({"type":"disconnect"}));
        });
        Ok((Self(inner), receiver))
    }
    pub fn send(&self, message: Value) -> Result<(), String> {
        self.0
            .output
            .send(message)
            .map_err(|_| "서비스 연결이 종료되었습니다".into())
    }
    pub fn notify(&self, name: &str, params: Value) {
        let _ = self.send(json!({"type":"event", "name":name, "params":params}));
    }
    pub async fn request(&self, method: &str, params: Value) -> Reply {
        let id = self.0.sequence.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.0.pending.lock().unwrap().insert(id, sender);
        if let Err(error) =
            self.send(json!({"type":"request", "id":id, "method":method, "params":params}))
        {
            self.0.pending.lock().unwrap().remove(&id);
            return Err(error);
        }
        let result = tokio::time::timeout(Duration::from_secs(240), receiver).await;
        self.0.pending.lock().unwrap().remove(&id);
        match result {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => Err("서비스 응답 연결이 종료되었습니다".into()),
            Err(_) => Err(format!("요청 시간이 초과되었습니다: {method}")),
        }
    }
    pub async fn invoke(&self, window_id: u64, channel: &str, args: Value) -> Reply {
        self.request(
            "invoke",
            json!({"windowId":window_id, "channel":channel, "args":args}),
        )
        .await
    }
}
