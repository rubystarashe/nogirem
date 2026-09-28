use crate::{Result, powershell};
use serde_json::{Value, json};
use std::time::Duration;
const QUERY: &str = include_str!("powershell/network-query.ps1");
const CONNECTIVITY: &str = include_str!("powershell/network-connectivity.ps1");
const RESTORE: &str = include_str!("powershell/network-restore.ps1");
const RESTART: &str = include_str!("powershell/network-restart.ps1");
const AUTO: &str = include_str!("powershell/network-autotuning.ps1");
pub fn same_interface(a: &Value, b: &Value) -> bool {
    let guid = |v: &Value| {
        v["interfaceGuid"]
            .as_str()
            .unwrap_or("")
            .trim()
            .replace(['{', '}'], "")
            .to_lowercase()
    };
    let ga = guid(a);
    let gb = guid(b);
    if !ga.is_empty() && !gb.is_empty() {
        return ga == gb;
    }
    let number = |v: &Value| {
        v["interfaceIndex"]
            .as_i64()
            .or_else(|| v["interfaceIndex"].as_str()?.parse().ok())
    };
    number(a)
        .zip(number(b))
        .is_some_and(|(a, b)| a > 0 && a == b)
}
pub fn configured(v: &Value) -> bool {
    v["supported"] != false && v["TcpAckFrequency"] == 1 && v["TCPNoDelay"] == 1
}
fn normal(v: &Value) -> bool {
    v["effective"]
        .as_str()
        .is_some_and(|s| s.eq_ignore_ascii_case("normal"))
}
pub trait Operations {
    fn query(&mut self, apply: bool) -> Result<Value>;
    fn autotuning(&mut self, apply: bool) -> Result<Value>;
    fn restart(&mut self, current: &Value) -> Result<Value>;
    fn restore(&mut self, target: &Value) -> Result<Value>;
}
pub struct Windows;
impl Operations for Windows {
    fn query(&mut self, apply: bool) -> Result<Value> {
        powershell::json(&format!("$applyFastPing = ${apply}\n{QUERY}"), 30000)
            .map_err(|e| format!("주 네트워크 인터페이스의 패스트핑 처리 실패: {e}"))
    }
    fn autotuning(&mut self, apply: bool) -> Result<Value> {
        powershell::json(&format!("$applyNormal = ${apply}\n{AUTO}"), 30000)
            .map_err(|e| format!("TCP 수신 창 자동 조정 처리 실패: {e}"))
    }
    fn restart(&mut self, current: &Value) -> Result<Value> {
        let index = current["interfaceIndex"]
            .as_u64()
            .filter(|i| *i > 0)
            .ok_or("잘못된 네트워크 인터페이스 인덱스")?;
        powershell::json(
            &RESTART.replace("${interfaceIndex}", &index.to_string()),
            30000,
        )
        .map_err(|e| format!("주 네트워크 인터페이스 재시작 실패: {e}"))
    }
    fn restore(&mut self, target: &Value) -> Result<Value> {
        let guid = target["interfaceGuid"].as_str().unwrap_or("");
        let bytes = guid.as_bytes();
        if bytes.len() != 38
            || bytes[0] != b'{'
            || bytes[37] != b'}'
            || !bytes[1..37]
                .iter()
                .all(|b| b.is_ascii_hexdigit() || *b == b'-')
        {
            return Err("복원할 네트워크 인터페이스 GUID가 올바르지 않습니다".into());
        }
        let index = target["interfaceIndex"]
            .as_u64()
            .filter(|i| *i > 0)
            .ok_or("복원할 네트워크 인터페이스 인덱스가 올바르지 않습니다")?;
        let literal = |name: &str| -> Result<String> {
            let v = &target[name];
            if v.is_null() {
                Ok("$null".into())
            } else {
                v.as_u64()
                    .filter(|n| *n <= u32::MAX as u64)
                    .map(|n| format!("([uint32]{n})"))
                    .ok_or("복원할 패스트핑 값이 올바르지 않습니다".into())
            }
        };
        let script = RESTORE
            .replace("${interfaceGuid}", guid)
            .replace("${interfaceIndex}", &index.to_string())
            .replace(
                "${valueLiteral(target.TcpAckFrequency)}",
                &literal("TcpAckFrequency")?,
            )
            .replace(
                "${valueLiteral(target.TCPNoDelay)}",
                &literal("TCPNoDelay")?,
            );
        powershell::json(&script, 30000).map_err(|e| format!("패스트핑 설정 복원 실패: {e}"))
    }
}
pub fn ensure_fast(ops: &mut impl Operations, apply: bool, restart: bool) -> Result<Value> {
    let before = ops.query(false)?;
    let mut result = json!({"supported":true,"configured":configured(&before),"applied":false,"restarted":false,"before":before,"current":before});
    if before["supported"] == false {
        result["supported"] = json!(false);
        result["configured"] = json!(false);
        result["reason"] = before
            .get("reason")
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(json!("특수 네트워크 환경으로 패스트핑 적용 생략"));
        return Ok(result);
    }
    if configured(&before) || !apply {
        return Ok(result);
    }
    if before["compatible"] == false {
        result["supported"] = json!(false);
        result["compatibilityBlocked"] = json!(true);
        result["reason"] = before
            .get("compatibilityReason")
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(json!("현재 네트워크 환경에는 패스트핑을 적용하지 않습니다"));
        return Ok(result);
    }
    let current = ops.query(true)?;
    if !configured(&current) {
        return Err("패스트핑 레지스트리 값을 적용한 뒤 검증에 실패했습니다".into());
    }
    let restarted = if restart {
        ops.restart(&current)?
    } else {
        Value::Null
    };
    Ok(
        json!({"supported":true,"configured":true,"applied":true,"restarted":!restarted.is_null(),"restart":restarted,"before":before,"current":current}),
    )
}
pub fn ensure_auto(ops: &mut impl Operations, apply: bool) -> Result<Value> {
    let before = ops.autotuning(false)?;
    if normal(&before) || !apply {
        return Ok(
            json!({"supported":true,"optimized":normal(&before),"applied":false,"before":before,"current":before}),
        );
    }
    let current = ops.autotuning(true)?;
    if !normal(&current) {
        return Err(format!(
            "TCP 자동 조정 수준을 Normal로 복구하지 못했습니다{}",
            if current["groupPolicy"] != "NotConfigured" {
                format!(
                    ", 그룹 정책={}",
                    current["groupPolicy"].as_str().unwrap_or("")
                )
            } else {
                String::new()
            }
        ));
    }
    Ok(json!({"supported":true,"optimized":true,"applied":true,"before":before,"current":current}))
}
pub fn restore_fast(ops: &mut impl Operations, target: &Value, restart: bool) -> Result<Value> {
    let current = ops.restore(target)?;
    if current["TcpAckFrequency"] != target["TcpAckFrequency"]
        || current["TCPNoDelay"] != target["TCPNoDelay"]
    {
        return Err("패스트핑 레지스트리 값을 복원한 뒤 검증에 실패했습니다".into());
    }
    let restarted = if restart {
        ops.restart(&current)?
    } else {
        Value::Null
    };
    Ok(
        json!({"supported":true,"configured":configured(&current),"restored":true,"restarted":!restarted.is_null(),"restart":restarted,"target":target,"current":current}),
    )
}
pub fn connectivity(attempts: u64, interval: u64) -> Result<Value> {
    let attempts = attempts.clamp(1, 20);
    let mut current = Value::Null;
    for attempt in 1..=attempts {
        current=powershell::json(CONNECTIVITY,15000).unwrap_or_else(|e|json!({"validIpv4":false,"defaultRoute":false,"gateway":false,"dns":false,"https":false,"error":e}));
        if current["validIpv4"] == true
            && current["defaultRoute"] == true
            && current["gateway"] == true
            && (current["dns"] == true || current["https"] == true)
        {
            return Ok(json!({"healthy":true,"attempts":attempt,"current":current}));
        }
        if attempt < attempts {
            std::thread::sleep(Duration::from_millis(interval));
        }
    }
    Ok(json!({"healthy":false,"attempts":attempts,"current":current}))
}
pub fn dispatch(action: &str, params: &Value) -> Result<Value> {
    let mut ops = Windows;
    match action {
        "nic-status" => crate::nic::status(),
        "nic-apply" => crate::nic::apply(params["applyChanges"] == true),
        "nic-restore" => crate::nic::restore(params["applyChanges"] == true),
        "fast-ping" => ensure_fast(
            &mut ops,
            params["applyChanges"] == true,
            params["restartAfterApply"] == true,
        ),
        "auto-tuning" => ensure_auto(&mut ops, params["applyChanges"] == true),
        "restore-fast-ping" => restore_fast(
            &mut ops,
            &params["target"],
            params["restartAfterRestore"] == true,
        ),
        "connectivity" => connectivity(
            params["attempts"].as_u64().unwrap_or(1),
            params["intervalMs"].as_u64().unwrap_or(3000),
        ),
        _ => Err(format!("Unknown network operation: {action}")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Mock {
        before: Value,
        after: Value,
        writes: usize,
        restarts: usize,
    }
    impl Operations for Mock {
        fn query(&mut self, apply: bool) -> Result<Value> {
            if apply {
                self.writes += 1;
                Ok(self.after.clone())
            } else {
                Ok(self.before.clone())
            }
        }
        fn autotuning(&mut self, apply: bool) -> Result<Value> {
            self.query(apply)
        }
        fn restart(&mut self, _: &Value) -> Result<Value> {
            self.restarts += 1;
            Ok(json!({"status":"Up"}))
        }
        fn restore(&mut self, v: &Value) -> Result<Value> {
            self.writes += 1;
            Ok(v.clone())
        }
    }
    #[test]
    fn incompatible_network_and_read_only_never_write() {
        for apply in [false, true] {
            let mut ops = Mock {
                before: json!({"compatible":false}),
                after: json!({}),
                writes: 0,
                restarts: 0,
            };
            assert_eq!(
                ensure_fast(&mut ops, apply, true).unwrap()["applied"],
                false
            );
            assert_eq!(ops.writes, 0);
            assert_eq!(ops.restarts, 0);
        }
    }
    #[test]
    fn apply_verify_then_restart() {
        let mut ops = Mock {
            before: json!({"compatible":true}),
            after: json!({"TcpAckFrequency":1,"TCPNoDelay":1}),
            writes: 0,
            restarts: 0,
        };
        assert_eq!(
            ensure_fast(&mut ops, true, true).unwrap()["restarted"],
            true
        );
        assert_eq!((ops.writes, ops.restarts), (1, 1));
    }
    #[test]
    fn failed_verification_never_restarts() {
        let mut ops = Mock {
            before: json!({}),
            after: json!({"TCPNoDelay":0}),
            writes: 0,
            restarts: 0,
        };
        assert!(ensure_fast(&mut ops, true, true).is_err());
        assert_eq!(ops.restarts, 0);
    }
}
