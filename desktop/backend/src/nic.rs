use crate::{Result, powershell, storage};
use base64::Engine;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
const SCRIPT: &str = include_str!("powershell/nic.ps1");
pub fn plan(status: &Value, count: u32) -> Result<Value> {
    if count < 4 || count % 2 != 0 {
        return Err(format!("지원하지 않는 논리 CPU 수입니다: {count}"));
    }
    Ok(
        json!({"interfaceAlias":status["interfaceAlias"],"interfaceIndex":status["interfaceIndex"],"processorGroup":0,"baseProcessorNumber":0,"maxProcessorNumber":count/2-1,"maxProcessors":count/2,"gameProcessorStart":count/2,"gameProcessorEnd":count-1,"profile":"ClosestStatic"}),
    )
}
pub fn optimized(status: &Value, target: &Value) -> bool {
    status["enabled"] == true
        && status["baseProcessorGroup"] == target["processorGroup"]
        && status["baseProcessorNumber"] == target["baseProcessorNumber"]
        && status["maxProcessorGroup"] == target["processorGroup"]
        && status["maxProcessorNumber"] == target["maxProcessorNumber"]
        && status["maxProcessors"] == target["maxProcessors"]
}
fn count() -> u32 {
    unsafe { windows_sys::Win32::System::Threading::GetActiveProcessorCount(0xffff) }
}
fn run(action: &str, payload: &Value) -> Result<Value> {
    if !matches!(action, "status" | "apply" | "restore") {
        return Err("Unknown NIC action".into());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.to_string());
    powershell::json(
        &SCRIPT
            .replace("${action}", action)
            .replace("${encodedPayload}", &encoded),
        30000,
    )
    .map_err(|e| format!("NIC RSS affinity 처리 실패: {e}"))
}
fn path() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA unavailable")?)
            .join("nogirem/nic-rss-state.json"),
    )
}
pub fn status() -> Result<Value> {
    let current = run("status", &json!({}))?;
    let target = plan(&current, count())?;
    Ok(
        json!({"supported":true,"optimized":optimized(&current,&target),"rssEnabled":current["enabled"],"gameCpuOverlap":current["enabled"]!=true||current["maxProcessorNumber"].is_null()||current["maxProcessorNumber"].as_u64().unwrap_or(0)>=target["gameProcessorStart"].as_u64().unwrap_or(0),"current":current,"target":target}),
    )
}
pub fn apply(changes: bool) -> Result<Value> {
    let mut before = status()?;
    if before["optimized"] == true || !changes {
        before["applied"] = json!(false);
        before["restarted"] = json!(false);
        return Ok(before);
    }
    let path = path()?;
    if storage::read_json(&path)?.is_none() {
        storage::write_json(&path, &before["current"])?;
    }
    let mut current = run("apply", &before["target"])?;
    for _ in 0..10 {
        if optimized(&current, &before["target"]) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
        current = run("status", &json!({}))?;
    }
    if !optimized(&current, &before["target"]) {
        return Err("NIC RSS affinity 적용 후 목표 CPU 범위를 확인하지 못했습니다".into());
    }
    Ok(
        json!({"supported":true,"optimized":true,"applied":true,"restarted":true,"before":before["current"],"current":current,"target":before["target"],"gameCpuOverlap":false}),
    )
}
pub fn restore(changes: bool) -> Result<Value> {
    let path = path()?;
    let Some(original) = storage::read_json(&path)? else {
        return Ok(
            json!({"supported":true,"restored":false,"reason":"복원할 NIC RSS 원본 설정이 없습니다"}),
        );
    };
    if !changes {
        return Ok(json!({"supported":true,"restored":false,"original":original}));
    }
    let matches = |v: &Value| {
        if original["enabled"] == true {
            v["enabled"] == true
                && v["baseProcessorNumber"] == original["baseProcessorNumber"]
                && v["maxProcessorNumber"] == original["maxProcessorNumber"]
        } else {
            v["enabled"] == false
        }
    };
    let before = run("status", &json!({}))?;
    let (current, restarted) = if matches(&before) {
        (before, false)
    } else {
        (run("restore", &original)?, true)
    };
    if !matches(&current) {
        return Err("NIC RSS affinity 원본 설정 복원에 실패했습니다".into());
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(
        json!({"supported":true,"restored":true,"restarted":restarted,"original":original,"current":current}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_rss_cpu_ranges() {
        let p = plan(&json!({"interfaceAlias":"이더넷","interfaceIndex":4}), 24).unwrap();
        assert_eq!(p["maxProcessorNumber"], 11);
        assert_eq!(p["gameProcessorStart"], 12);
        let s = json!({"enabled":true,"baseProcessorGroup":0,"baseProcessorNumber":0,"maxProcessorGroup":0,"maxProcessorNumber":11,"maxProcessors":12});
        assert!(optimized(&s, &p));
        let mut disabled = s.clone();
        disabled["enabled"] = json!(false);
        assert!(!optimized(&disabled, &p));
        assert!(plan(&s, 3).is_err());
    }
}
