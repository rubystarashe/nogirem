use crate::{
    Result,
    network::{self, Operations},
    storage,
    workers::{read, remove},
};
use serde_json::{Value, json};
use std::path::Path;

pub trait Connection {
    fn check(&mut self, attempts: u64, interval: u64) -> Result<Value>;
}
pub struct WindowsConnection;
impl Connection for WindowsConnection {
    fn check(&mut self, attempts: u64, interval: u64) -> Result<Value> {
        network::connectivity(attempts, interval)
    }
}
pub fn status(ops: &mut impl Operations, state: &Path) -> Result<Value> {
    let fast = network::ensure_fast(ops, false, false)?;
    let auto = network::ensure_auto(ops, false)?;
    Ok(
        json!({"originalStateRecorded":!read(state).is_null(),"optimized":(fast["supported"]==false||fast["configured"]==true)&&auto["optimized"]==true,"fastPing":fast,"tcpAutoTuning":auto}),
    )
}
fn snapshot(current: &Value) -> Value {
    json!({"interfaceAlias":current["interfaceAlias"],"interfaceIndex":current["interfaceIndex"],"interfaceGuid":current["interfaceGuid"],"TcpAckFrequency":current["TcpAckFrequency"],"TCPNoDelay":current["TCPNoDelay"]})
}
pub fn apply(
    ops: &mut impl Operations,
    connection: &mut impl Connection,
    state: &Path,
) -> Result<Value> {
    let before = network::ensure_fast(ops, false, false)?;
    if before["configured"] != true && before["current"]["compatible"] == false {
        return Err(before["current"]["compatibilityReason"]
            .as_str()
            .unwrap_or("현재 네트워크 환경에는 패스트핑을 적용하지 않습니다")
            .into());
    }
    if connection.check(1, 3000)?["healthy"] != true {
        return Err("현재 인터넷 연결이 정상적이지 않아 패스트핑을 적용하지 않았습니다".into());
    }
    if before["supported"] != false
        && before["configured"] != true
        && !network::same_interface(&read(state), &before["current"])
    {
        let mut saved = snapshot(&before["current"]);
        saved["version"] = json!(1);
        saved["capturedAt"] = json!(crate::now_ms());
        storage::write_json(state, &saved)?;
    }
    let auto = network::ensure_auto(ops, true)?;
    let fast = network::ensure_fast(ops, true, true)?;
    let connectivity = connection.check(10, 3000)?;
    if connectivity["healthy"] != true && fast["configured"] == true {
        let saved = read(state);
        let target = if saved["interfaceGuid"].is_string()
            && network::same_interface(&saved, &fast["current"])
        {
            saved
        } else {
            let mut v = snapshot(&fast["current"]);
            v["TcpAckFrequency"] = before["current"]["TcpAckFrequency"].clone();
            v["TCPNoDelay"] = before["current"]["TCPNoDelay"].clone();
            v
        };
        let rollback = network::restore_fast(ops, &target, true)?;
        remove(state)?;
        return Ok(
            json!({"fastPing":rollback,"tcpAutoTuning":auto,"connectivity":connectivity,"rolledBack":true,"optimized":false}),
        );
    }
    Ok(
        json!({"optimized":(fast["supported"]==false||fast["configured"]==true)&&auto["optimized"]==true,"fastPing":fast,"tcpAutoTuning":auto,"connectivity":connectivity,"rolledBack":false}),
    )
}
pub fn restore(ops: &mut impl Operations, state: &Path) -> Result<Value> {
    let saved = read(state);
    let before = network::ensure_fast(ops, false, false)?;
    let has_saved = saved["interfaceGuid"].is_string()
        && saved["interfaceIndex"]
            .as_i64()
            .or_else(|| {
                saved["interfaceIndex"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
            })
            .is_some();
    if before["supported"] == false && !has_saved {
        let auto = network::ensure_auto(ops, false)?;
        return Ok(
            json!({"fastPing":before,"originalStateRecorded":!saved.is_null(),"restored":false,"optimized":auto["optimized"],"tcpAutoTuning":auto}),
        );
    }
    let current = &before["current"];
    let matched = has_saved && !current.is_null() && network::same_interface(&saved, current);
    let target = if matched || current.is_null() {
        saved
    } else {
        let mut v = snapshot(current);
        v["TcpAckFrequency"] = Value::Null;
        v["TCPNoDelay"] = Value::Null;
        v
    };
    let fast = network::restore_fast(ops, &target, !current.is_null())?;
    remove(state)?;
    let auto = network::ensure_auto(ops, false)?;
    Ok(
        json!({"originalStateRecorded":has_saved,"restored":true,"restoredFromRecord":matched||(current.is_null()&&has_saved),"staleOriginalState":has_saved&&!current.is_null()&&!matched,"optimized":fast["configured"]==true&&auto["optimized"]==true,"fastPing":fast,"tcpAutoTuning":auto}),
    )
}
pub fn dispatch(action: &str, state: &Path) -> Result<Value> {
    let mut ops = network::Windows;
    match action {
        "status" => status(&mut ops, state),
        "apply" => apply(&mut ops, &mut WindowsConnection, state),
        "restore" => restore(&mut ops, state),
        _ => Err("잘못된 네트워크 요청".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        current: Value,
        writes: usize,
        restarts: usize,
        restores: usize,
    }
    impl Operations for Fake {
        fn query(&mut self, apply: bool) -> Result<Value> {
            if apply {
                self.writes += 1;
                self.current["TcpAckFrequency"] = json!(1);
                self.current["TCPNoDelay"] = json!(1);
            }
            Ok(self.current.clone())
        }
        fn autotuning(&mut self, _: bool) -> Result<Value> {
            Ok(json!({"effective":"normal"}))
        }
        fn restart(&mut self, _: &Value) -> Result<Value> {
            self.restarts += 1;
            Ok(json!({}))
        }
        fn restore(&mut self, v: &Value) -> Result<Value> {
            self.restores += 1;
            self.current = v.clone();
            Ok(v.clone())
        }
    }
    struct Health(Vec<bool>);
    impl Connection for Health {
        fn check(&mut self, _: u64, _: u64) -> Result<Value> {
            Ok(json!({"healthy":self.0.remove(0)}))
        }
    }
    fn fixture() -> (Fake, std::path::PathBuf) {
        (
            Fake {
                current: json!({"interfaceGuid":"{old}","interfaceIndex":3,"TcpAckFrequency":2,"TCPNoDelay":null}),
                writes: 0,
                restarts: 0,
                restores: 0,
            },
            std::env::temp_dir().join(format!("nogirem-network-{}.json", uuid::Uuid::new_v4())),
        )
    }
    #[test]
    fn unhealthy_baseline_does_not_write() {
        let (mut fake, path) = fixture();
        assert!(apply(&mut fake, &mut Health(vec![false]), &path).is_err());
        assert_eq!(fake.writes, 0);
        assert!(!path.exists());
    }
    #[test]
    fn failed_connection_restores_captured_values() {
        let (mut fake, path) = fixture();
        let result = apply(&mut fake, &mut Health(vec![true, false]), &path).unwrap();
        assert_eq!(result["rolledBack"], true);
        assert_eq!(fake.current["TcpAckFrequency"], 2);
        assert!(fake.current["TCPNoDelay"].is_null());
        assert_eq!(fake.restarts, 2);
        assert_eq!(fake.restores, 1);
        assert!(!path.exists());
    }
    #[test]
    fn unrelated_adapter_record_is_not_applied() {
        let (mut fake, path) = fixture();
        storage::write_json(&path,&json!({"interfaceGuid":"{unrelated}","interfaceIndex":9,"TcpAckFrequency":9,"TCPNoDelay":9})).unwrap();
        let result = restore(&mut fake, &path).unwrap();
        assert_eq!(result["staleOriginalState"], true);
        assert_eq!(fake.current["interfaceGuid"], "{old}");
        assert!(fake.current["TcpAckFrequency"].is_null());
        assert!(!path.exists());
    }
}
