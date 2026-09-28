use crate::Result;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub struct Output {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
fn drain(
    mut reader: impl Read,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if bytes.len() + n > limit {
            overflow.store(true, Ordering::Relaxed)
        } else {
            bytes.extend_from_slice(&chunk[..n])
        }
    }
    Ok(bytes)
}
pub fn run(
    exe: impl AsRef<Path>,
    args: &[String],
    timeout_ms: u64,
    limit: usize,
) -> Result<Output> {
    run_progress(exe, args, timeout_ms, limit, None)
}
pub fn run_progress(
    exe: impl AsRef<Path>,
    args: &[String],
    timeout_ms: u64,
    limit: usize,
    callback: Option<Arc<dyn Fn(u32) + Send + Sync>>,
) -> Result<Output> {
    use std::os::windows::process::CommandExt;
    let mut child = Command::new(exe.as_ref())
        .args(args)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.as_ref().display()))?;
    let out = child.stdout.take().ok_or("stdout missing")?;
    let err = child.stderr.take().ok_or("stderr missing")?;
    let overflow = Arc::new(AtomicBool::new(false));
    let f = overflow.clone();
    let out = std::thread::spawn(move || drain(out, limit, f));
    let f = overflow.clone();
    let err = std::thread::spawn(move || {
        use std::io::BufRead;
        let mut bytes = Vec::new();
        let mut reader = std::io::BufReader::new(err);
        let mut chunk = Vec::new();
        loop {
            chunk.clear();
            let n = reader
                .by_ref()
                .take(limit as u64 + 1)
                .read_until(b'\n', &mut chunk)?;
            if n == 0 {
                break;
            }
            if n > limit {
                f.store(true, Ordering::Relaxed);
                continue;
            }
            let text = String::from_utf8_lossy(&chunk);
            if let Some(p) = text
                .trim()
                .strip_prefix("PROGRESS ")
                .and_then(|s| s.parse::<u32>().ok())
            {
                if let Some(c) = &callback {
                    c(p.min(100));
                }
            } else if bytes.len() + n > limit {
                f.store(true, Ordering::Relaxed);
            } else {
                bytes.extend_from_slice(&chunk)
            }
        }
        Ok::<_, std::io::Error>(bytes)
    });
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut failure = None;
    let status = loop {
        if Instant::now() >= deadline || overflow.load(Ordering::Relaxed) {
            failure = Some("프로세스 시간 또는 출력 제한 초과".to_string());
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                failure = Some(e.to_string());
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "stdout reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "stderr reader failed")?
        .map_err(|e| e.to_string())?;
    if overflow.load(Ordering::Relaxed) {
        return Err("프로세스 출력 제한 초과".into());
    }
    if let Some(e) = failure {
        return Err(e);
    }
    Ok(Output {
        success: status.is_some_and(|s| s.success()),
        code: status.and_then(|s| s.code()),
        stdout,
        stderr,
    })
}
pub fn text(exe: impl AsRef<Path>, args: &[String], timeout_ms: u64) -> Result<String> {
    let o = run(exe, args, timeout_ms, 1024 * 1024)?;
    if !o.success {
        return Err(format!(
            "프로세스 종료 {:?}: {}",
            o.code,
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .into())
}
