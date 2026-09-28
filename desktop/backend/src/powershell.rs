use crate::Result;
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
fn executable() -> PathBuf {
    for name in ["SystemRoot", "SYSTEMROOT", "windir", "WINDIR"] {
        if let Some(root) = std::env::var_os(name) {
            let p = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from("powershell.exe")
}
fn capture(
    mut reader: impl Read,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> std::io::Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if result.len() + read > limit {
            overflow.store(true, Ordering::Relaxed);
        } else {
            result.extend_from_slice(&buffer[..read]);
        }
    }
    Ok(result)
}
pub fn run(script: &str, timeout_ms: u64, max_bytes: usize) -> Result<String> {
    use std::os::windows::process::CommandExt;
    let mut child=Command::new(executable()).args(["-NoProfile","-NonInteractive","-Command","[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);$reader=[IO.StreamReader]::new([Console]::OpenStandardInput(),[Text.UTF8Encoding]::new($false),$false);& ([ScriptBlock]::Create($reader.ReadToEnd()))"])
        .creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("PowerShell stdin missing")?;
    let text = script.to_owned();
    let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
    let stdout = child.stdout.take().ok_or("PowerShell stdout missing")?;
    let stderr = child.stderr.take().ok_or("PowerShell stderr missing")?;
    let overflow = Arc::new(AtomicBool::new(false));
    let flag = overflow.clone();
    let out = std::thread::spawn(move || capture(stdout, max_bytes, flag));
    let flag = overflow.clone();
    let err = std::thread::spawn(move || capture(stderr, max_bytes, flag));
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut failure = None;
    let status = loop {
        if overflow.load(Ordering::Relaxed) {
            failure = Some("PowerShell 출력이 허용 크기를 초과했습니다".to_string());
            let _ = child.kill();
        } else if Instant::now() >= deadline {
            failure = Some(format!(
                "PowerShell 실행 제한 시간 {timeout_ms}ms를 초과했습니다"
            ));
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                failure = Some(e.to_string());
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = writer.join();
    let stdout = out
        .join()
        .map_err(|_| "PowerShell stdout reader panicked")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "PowerShell stderr reader panicked")?
        .map_err(|e| e.to_string())?;
    if overflow.load(Ordering::Relaxed) {
        return Err("PowerShell 출력이 허용 크기를 초과했습니다".into());
    }
    if let Some(e) = failure {
        return Err(e);
    }
    if !status.is_some_and(|s| s.success()) {
        return Err(String::from_utf8_lossy(&stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .to_owned())
}
pub fn json(script: &str, timeout_ms: u64) -> Result<serde_json::Value> {
    serde_json::from_str(&run(script, timeout_ms, 1024 * 1024)?).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_output_and_nonzero_exit() {
        assert_eq!(run("Write-Output '한글'", 5000, 1024).unwrap(), "한글");
        assert!(run("throw 'failed'", 5000, 4096).is_err());
    }
    #[test]
    fn bounded_execution() {
        assert!(
            run("Start-Sleep -Seconds 5", 100, 1024)
                .unwrap_err()
                .contains("100ms")
        );
        assert!(
            run("Write-Output ('x' * 4096)", 5000, 100)
                .unwrap_err()
                .contains("크기")
        );
    }
}
