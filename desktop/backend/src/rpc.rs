use crate::Result;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};
struct Inner {
    sequence: AtomicU64,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Value>>>>,
    output: mpsc::Sender<Value>,
    closed: AtomicBool,
}
#[derive(Clone)]
pub struct Rpc(Arc<Inner>);
impl Rpc {
    pub fn stdio() -> (Self, mpsc::Receiver<Value>) {
        let (output, writes) = mpsc::channel::<Value>();
        let (events, receiver) = mpsc::channel();
        let inner = Arc::new(Inner {
            sequence: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            output,
            closed: AtomicBool::new(false),
        });
        std::thread::spawn(move || {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            for v in writes {
                if writeln!(stdout, "{v}")
                    .and_then(|_| stdout.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let weak = Arc::downgrade(&inner);
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if value["type"] == "response" {
                    if let Some(inner) = weak.upgrade() {
                        if let Some(id) = value["id"].as_u64() {
                            if let Some(reply) = inner.pending.lock().unwrap().remove(&id) {
                                let _ = reply.send(if let Some(e) = value["error"].as_str() {
                                    Err(e.into())
                                } else {
                                    Ok(value["result"].clone())
                                });
                            }
                        }
                    }
                } else if events.send(value).is_err() {
                    break;
                }
            }
            if let Some(inner) = weak.upgrade() {
                inner.closed.store(true, Ordering::SeqCst);
                for (_, reply) in inner.pending.lock().unwrap().drain() {
                    let _ = reply.send(Err("창 호스트 연결이 종료되었습니다".into()));
                }
            }
            let _ = events.send(json!({"type":"disconnect"}));
        });
        (Self(inner), receiver)
    }
    pub fn closed(&self) -> bool {
        self.0.closed.load(Ordering::SeqCst)
    }
    pub fn send(&self, message: Value) {
        let _ = self.0.output.send(message);
    }
    pub fn request(&self, method: &str, params: Value) -> Result<Value> {
        if self.closed() {
            return Err("창 호스트 연결이 종료되었습니다".into());
        }
        if std::env::var_os("NOGIREM_SERVICE_TRACE").is_some() {
            eprintln!("native request: {method}");
        }
        let id = self.0.sequence.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::channel();
        self.0.pending.lock().unwrap().insert(id, sender);
        self.send(json!({"type":"request","id":id,"method":method,"params":params}));
        let result = receiver
            .recv_timeout(Duration::from_secs(240))
            .map_err(|e| format!("{method}: {e}"));
        self.0.pending.lock().unwrap().remove(&id);
        if std::env::var_os("NOGIREM_SERVICE_TRACE").is_some() {
            eprintln!("native response: {method}");
        }
        result?
    }
    pub fn native(&self, method: &str, params: Value) {
        self.send(
            json!({"type":"event","name":"native","params":{"method":method,"params":params}}),
        );
    }
    pub fn event(&self, id: u64, channel: &str, args: Value) {
        self.native(
            "window.event",
            json!({"windowId":id,"channel":channel,"args":args}),
        );
    }
    pub fn reply(&self, id: &Value, result: Result<Value>) {
        self.send(match result {
            Ok(v) => json!({"type":"response","id":id,"result":v}),
            Err(e) => json!({"type":"response","id":id,"error":e}),
        });
    }
}
