#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        SystemInformation::{GetSystemDirectoryW, GetWindowsDirectoryW},
        Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
            QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        },
    },
};

type Result<T> = std::result::Result<T, String>;
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct ProcessRecord {
    pid: u32,
    parent_pid: u32,
}
struct OwnedProcess {
    pid: u32,
    handle: Handle,
}
struct CleanupOptions {
    install_directory: PathBuf,
    caller_pid: u32,
    updater_pid: u32,
    uninstall: bool,
    timeout: Duration,
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn snapshot() -> Result<Vec<ProcessRecord>> {
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let handle = Handle(handle);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut records = Vec::new();
    let mut present = unsafe { Process32FirstW(handle.0, &mut entry) };
    while present != 0 {
        records.push(ProcessRecord {
            pid: entry.th32ProcessID,
            parent_pid: entry.th32ParentProcessID,
        });
        present = unsafe { Process32NextW(handle.0, &mut entry) };
    }
    Ok(records)
}
fn open_process(pid: u32, terminate: bool) -> Result<Handle> {
    let access = PROCESS_QUERY_LIMITED_INFORMATION
        | PROCESS_SYNCHRONIZE
        | if terminate { PROCESS_TERMINATE } else { 0 };
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(Handle(handle))
    }
}
fn process_path(handle: &Handle) -> Result<PathBuf> {
    let mut data = vec![0u16; 32768];
    let mut length = data.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, data.as_mut_ptr(), &mut length) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &data[..length as usize],
    )))
}
fn normalized(path: &Path) -> String {
    let value = path
        .as_os_str()
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase();
    value
        .strip_prefix(r"\\?\")
        .or_else(|| value.strip_prefix(r"\??\"))
        .unwrap_or(&value)
        .to_owned()
}
fn same_path(left: &Path, right: &Path) -> bool {
    normalized(left) == normalized(right)
}
fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = normalized(path);
    let root = normalized(root);
    path == root || path.starts_with(&(root + "\\"))
}
fn canonical_existing(path: &Path, label: &str) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(format!("{label} 경로가 절대 경로가 아닙니다"));
    }
    path.canonicalize().map_err(error)
}
fn processes_in(root: &Path, excluded: &[u32], terminate: bool) -> Result<Vec<OwnedProcess>> {
    let mut result = Vec::new();
    for record in snapshot()? {
        if record.pid == 0 || excluded.contains(&record.pid) {
            continue;
        }
        let Ok(handle) = open_process(record.pid, terminate) else {
            continue;
        };
        let Ok(path) = process_path(&handle) else {
            continue;
        };
        if path_is_within(&path, root) {
            result.push(OwnedProcess {
                pid: record.pid,
                handle,
            });
        }
    }
    Ok(result)
}
fn updater_process(pid: u32, updater_root: &Path) -> Option<OwnedProcess> {
    let Ok(handle) = open_process(pid, false) else {
        return None;
    };
    let Ok(path) = process_path(&handle) else {
        return None;
    };
    (path_is_within(&path, updater_root)
        && path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("updater.exe")))
    .then_some(OwnedProcess { pid, handle })
}
fn preserved_updater(
    explicit_pid: u32,
    caller_pid: u32,
    updater_root: &Path,
) -> Option<OwnedProcess> {
    if let Some(process) = updater_process(explicit_pid, updater_root) {
        return Some(process);
    }
    let parent = snapshot()
        .ok()?
        .into_iter()
        .find(|record| record.pid == caller_pid)?
        .parent_pid;
    updater_process(parent, updater_root)
}
fn terminate(process: &OwnedProcess) {
    unsafe { TerminateProcess(process.handle.0, 1) };
}
fn process_alive(process: &OwnedProcess) -> bool {
    unsafe { WaitForSingleObject(process.handle.0, 0) == WAIT_TIMEOUT }
}
fn exclusions(
    own_pid: u32,
    caller: Option<&OwnedProcess>,
    updater: Option<&OwnedProcess>,
) -> Vec<u32> {
    let mut result = vec![own_pid];
    if let Some(process) = caller.filter(|process| process_alive(process)) {
        result.push(process.pid);
    }
    if let Some(process) = updater.filter(|process| process_alive(process)) {
        result.push(process.pid);
    }
    result
}
fn wait_processes_gone(
    root: &Path,
    own_pid: u32,
    caller: Option<&OwnedProcess>,
    updater: Option<&OwnedProcess>,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if processes_in(root, &exclusions(own_pid, caller, updater), false)?.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("대상 프로세스를 종료하지 못했습니다".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}
fn decode_task_xml(bytes: &[u8]) -> Result<String> {
    let utf16_le = bytes.starts_with(&[0xff, 0xfe])
        || (bytes.len().is_multiple_of(2)
            && bytes.chunks_exact(2).filter(|chunk| chunk[1] == 0).count() > bytes.len() / 4);
    if utf16_le {
        let start = if bytes.starts_with(&[0xff, 0xfe]) {
            2
        } else {
            0
        };
        let words = bytes[start..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&words).map_err(error);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        let words = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&words).map_err(error);
    }
    String::from_utf8(bytes.to_vec()).map_err(error)
}
fn task_command(xml: &str) -> Option<String> {
    let start = xml.find("<Command>")? + "<Command>".len();
    let end = xml[start..].find("</Command>")? + start;
    Some(
        xml[start..end]
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .trim()
            .trim_matches('"')
            .to_owned(),
    )
}
fn trusted_windows_directory(system: bool) -> Result<PathBuf> {
    let mut data = vec![0u16; 32768];
    let length = unsafe {
        if system {
            GetSystemDirectoryW(data.as_mut_ptr(), data.len() as u32)
        } else {
            GetWindowsDirectoryW(data.as_mut_ptr(), data.len() as u32)
        }
    };
    if length == 0 || length as usize >= data.len() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &data[..length as usize],
    )))
}
fn remove_startup_task(install_root: &Path) -> Result<()> {
    let task_file =
        trusted_windows_directory(false)?.join("System32/Tasks/Mabinogi Rem Booster Startup");
    let bytes = match fs::read(&task_file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    let xml = decode_task_xml(&bytes)?;
    let Some(command) = task_command(&xml) else {
        return Err("시작 예약 작업의 실행 경로를 확인하지 못했습니다".into());
    };
    if !same_path(Path::new(&command), &install_root.join("nogirem.exe")) {
        return Ok(());
    }
    let status = Command::new(trusted_windows_directory(true)?.join("schtasks.exe"))
        .args([
            OsStr::new("/Delete"),
            OsStr::new("/TN"),
            OsStr::new("Mabinogi Rem Booster Startup"),
            OsStr::new("/F"),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(error)?;
    if !status.success() {
        return Err(format!("시작 예약 작업을 제거하지 못했습니다: {status}"));
    }
    Ok(())
}
fn cleanup_with_roots(
    options: &CleanupOptions,
    install_root: &Path,
    updater_root: &Path,
    user_root: &Path,
) -> Result<()> {
    let install_directory = canonical_existing(&options.install_directory, "설치")?;
    if !same_path(&install_directory, install_root) {
        return Err("허용되지 않은 설치 경로입니다".into());
    }
    if options.uninstall {
        remove_startup_task(&install_directory)?;
    }
    let own_pid = std::process::id();
    let caller = open_process(options.caller_pid, false)
        .ok()
        .map(|handle| OwnedProcess {
            pid: options.caller_pid,
            handle,
        });
    let preserved = preserved_updater(options.updater_pid, options.caller_pid, updater_root);
    let stale_updaters = processes_in(
        updater_root,
        &exclusions(own_pid, caller.as_ref(), preserved.as_ref()),
        true,
    )?;
    for process in &stale_updaters {
        terminate(process);
    }
    drop(stale_updaters);
    wait_processes_gone(
        updater_root,
        own_pid,
        caller.as_ref(),
        preserved.as_ref(),
        Duration::from_secs(5),
    )?;

    let request_directory = user_root.join("instance");
    let request_file = request_directory.join("installer-close-request");
    fs::create_dir_all(&request_directory).map_err(error)?;
    let deadline = Instant::now() + options.timeout;
    let mut force_deadline = None;
    let mut quiet_since = None;
    let mut last_request = None;
    loop {
        let running = processes_in(
            &install_directory,
            &exclusions(own_pid, caller.as_ref(), None),
            true,
        )?;
        if running.is_empty() {
            let quiet = quiet_since.get_or_insert_with(Instant::now);
            if quiet.elapsed() >= Duration::from_secs(1) {
                break;
            }
        } else {
            quiet_since = None;
            if last_request.is_none_or(|last: Instant| last.elapsed() >= Duration::from_secs(1)) {
                let nonce = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                fs::write(
                    &request_file,
                    format!("installer-upgrade:{}-{nonce}", std::process::id()),
                )
                .map_err(error)?;
                last_request = Some(Instant::now());
            }
            if Instant::now() >= deadline {
                for process in &running {
                    terminate(process);
                }
                force_deadline.get_or_insert_with(|| Instant::now() + Duration::from_secs(5));
                if force_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    return Err("설치 폴더의 앱 프로세스를 종료하지 못했습니다".into());
                }
            }
        }
        drop(running);
        thread::sleep(Duration::from_millis(100));
    }
    match fs::remove_file(&request_file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
fn cleanup(options: &CleanupOptions) -> Result<()> {
    let install_root = canonical_existing(&options.install_directory, "설치")?;
    if !install_root.join("nogirem.exe").is_file()
        && !install_root.join("Uninstall.exe").is_file()
        && !(install_root.join("runtime/node.exe").is_file()
            && install_root.join("service/main.mjs").is_file())
    {
        return Err("렘 부스터 설치 경로를 확인하지 못했습니다".into());
    }
    let local_app_data =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA 누락")?);
    let app_data = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA 누락")?);
    cleanup_with_roots(
        options,
        &install_root,
        &local_app_data.join("NogiremUpdater/updates"),
        &app_data.join("마비노기 렘 부스터"),
    )
}
fn portable_cache_in_use(target: &Path) -> Result<bool> {
    if !target.is_absolute() {
        return Err("포터블 캐시 실행 파일 경로가 절대 경로가 아닙니다".into());
    }
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    for record in snapshot()? {
        if record.pid == 0 || record.pid == std::process::id() {
            continue;
        }
        let Ok(handle) = open_process(record.pid, false) else {
            continue;
        };
        if process_path(&handle).is_ok_and(|path| same_path(&path, &target))
            && unsafe { WaitForSingleObject(handle.0, 0) == WAIT_TIMEOUT }
        {
            return Ok(true);
        }
    }
    Ok(false)
}
fn values(args: &[String]) -> HashMap<&str, &str> {
    let mut values = HashMap::new();
    let mut index = 0;
    while index + 1 < args.len() {
        if args[index].starts_with("--") {
            values.insert(args[index].as_str(), args[index + 1].as_str());
            index += 2;
        } else {
            index += 1;
        }
    }
    values
}
fn parse_cleanup(args: &[String]) -> Result<CleanupOptions> {
    let values = values(args);
    let install_directory = PathBuf::from(
        *values
            .get("--install-directory")
            .ok_or("설치 경로 인자 누락")?,
    );
    let caller_pid = values
        .get("--caller-pid")
        .ok_or("호출 프로세스 인자 누락")?
        .parse::<u32>()
        .map_err(error)?;
    let updater_pid = values
        .get("--updater-pid")
        .copied()
        .unwrap_or("0")
        .parse::<u32>()
        .map_err(error)?;
    let timeout = values
        .get("--timeout-seconds")
        .copied()
        .unwrap_or("30")
        .parse::<u64>()
        .map_err(error)?;
    if !(1..=300).contains(&timeout) {
        return Err("종료 제한 시간은 1초부터 300초까지여야 합니다".into());
    }
    Ok(CleanupOptions {
        install_directory,
        caller_pid,
        updater_pid,
        uninstall: args.iter().any(|arg| arg == "--uninstall"),
        timeout: Duration::from_secs(timeout),
    })
}
fn execute(args: &[String]) -> Result<i32> {
    if let Some(index) = args.iter().position(|arg| arg == "--portable-cache-in-use") {
        let target = args.get(index + 1).ok_or("포터블 캐시 경로 누락")?;
        return portable_cache_in_use(Path::new(target)).map(|in_use| if in_use { 2 } else { 0 });
    }
    if args.iter().any(|arg| arg == "--cleanup") {
        cleanup(&parse_cleanup(args)?)?;
        return Ok(0);
    }
    Err("지원하지 않는 installer cleanup 작업입니다".into())
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match execute(&args) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Child;

    fn stop(child: &mut Child) {
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    fn spawn_copy(path: &Path) -> Child {
        Command::new(path)
            .args(["--ignored", "--exact", "tests::helper_process_fixture"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    #[test]
    #[ignore]
    fn helper_process_fixture() {
        thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore]
    fn legacy_updater_fixture() {
        let installer = std::env::var_os("NOGIREM_TEST_INSTALLER").unwrap();
        let state = std::env::var_os("NOGIREM_TEST_STATE").unwrap();
        let mut child = spawn_copy(Path::new(&installer));
        fs::write(state, child.id().to_string()).unwrap();
        let _ = child.wait();
    }

    #[test]
    #[ignore]
    fn graceful_close_fixture() {
        let request = PathBuf::from(std::env::var_os("NOGIREM_TEST_CLOSE_REQUEST").unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        while !request.is_file() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn 경로_경계는_형제_디렉터리를_포함하지_않는다() {
        assert!(path_is_within(
            Path::new(r"C:\Program Files\Nogirem\nogirem.exe"),
            Path::new(r"C:\Program Files\Nogirem")
        ));
        assert!(!path_is_within(
            Path::new(r"C:\Program Files\NogiremOther\other.exe"),
            Path::new(r"C:\Program Files\Nogirem")
        ));
    }

    #[test]
    fn 종료_옵션은_필수값과_제한_시간을_검증한다() {
        let args = vec![
            "--cleanup".into(),
            "--install-directory".into(),
            r"C:\Program Files\Nogirem".into(),
            "--caller-pid".into(),
            "42".into(),
            "--timeout-seconds".into(),
            "301".into(),
        ];
        assert!(parse_cleanup(&args).is_err());
    }

    #[test]
    fn 포터블_캐시는_정확히_같은_실행_경로만_사용_중으로_판정한다() {
        let root =
            std::env::temp_dir().join(format!("nogirem-portable-cleanup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let target = root.join("nogirem.exe");
        fs::copy(std::env::current_exe().unwrap(), &target).unwrap();
        let mut child = spawn_copy(&target);
        thread::sleep(Duration::from_millis(200));
        assert!(portable_cache_in_use(&target).unwrap());
        assert!(!portable_cache_in_use(&root.join("different.exe")).unwrap());
        stop(&mut child);
        assert!(!portable_cache_in_use(&target).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup은_설치_프로세스와_stale_updater만_종료한다() {
        let root =
            std::env::temp_dir().join(format!("nogirem-installer-cleanup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let install = root.join("install");
        let updater_root = root.join("local/NogiremUpdater/updates/job-test");
        let user = root.join("roaming/마비노기 렘 부스터");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&updater_root).unwrap();
        let app = install.join("nogirem.exe");
        let updater = updater_root.join("updater.exe");
        fs::copy(std::env::current_exe().unwrap(), &app).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &updater).unwrap();
        let mut app_child = spawn_copy(&app);
        let mut updater_child = spawn_copy(&updater);
        let late_app = app.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            sender.send(spawn_copy(&late_app)).unwrap();
        });
        let options = CleanupOptions {
            install_directory: install.canonicalize().unwrap(),
            caller_pid: std::process::id(),
            updater_pid: 0,
            uninstall: false,
            timeout: Duration::from_secs(1),
        };
        cleanup_with_roots(
            &options,
            &install.canonicalize().unwrap(),
            &root.join("local/NogiremUpdater/updates"),
            &user,
        )
        .unwrap();
        let mut late_child = receiver.recv().unwrap();
        assert!(app_child.try_wait().unwrap().is_some());
        assert!(late_child.try_wait().unwrap().is_some());
        assert!(updater_child.try_wait().unwrap().is_some());
        stop(&mut app_child);
        stop(&mut late_child);
        stop(&mut updater_child);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn 명시한_updater_pid는_종료하지_않는다() {
        let root =
            std::env::temp_dir().join(format!("nogirem-kept-updater-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let install = root.join("install");
        let updater_root = root.join("local/NogiremUpdater/updates/job-test");
        let user = root.join("roaming/마비노기 렘 부스터");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&updater_root).unwrap();
        let updater = updater_root.join("updater.exe");
        fs::copy(std::env::current_exe().unwrap(), &updater).unwrap();
        let mut updater_child = spawn_copy(&updater);
        let options = CleanupOptions {
            install_directory: install.canonicalize().unwrap(),
            caller_pid: std::process::id(),
            updater_pid: updater_child.id(),
            uninstall: false,
            timeout: Duration::from_secs(1),
        };
        cleanup_with_roots(
            &options,
            &install.canonicalize().unwrap(),
            &root.join("local/NogiremUpdater/updates"),
            &user,
        )
        .unwrap();
        assert!(updater_child.try_wait().unwrap().is_none());
        stop(&mut updater_child);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn 정상_종료_요청을_받은_앱은_강제_종료하지_않는다() {
        let root =
            std::env::temp_dir().join(format!("nogirem-graceful-cleanup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let install = root.join("install");
        let updater_root = root.join("local/NogiremUpdater/updates");
        let user = root.join("roaming/마비노기 렘 부스터");
        let request = user.join("instance/installer-close-request");
        fs::create_dir_all(&install).unwrap();
        let app = install.join("nogirem.exe");
        fs::copy(std::env::current_exe().unwrap(), &app).unwrap();
        let mut child = Command::new(&app)
            .args(["--ignored", "--exact", "tests::graceful_close_fixture"])
            .env("NOGIREM_TEST_CLOSE_REQUEST", &request)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        cleanup_with_roots(
            &CleanupOptions {
                install_directory: install.canonicalize().unwrap(),
                caller_pid: std::process::id(),
                updater_pid: 0,
                uninstall: false,
                timeout: Duration::from_secs(5),
            },
            &install.canonicalize().unwrap(),
            &updater_root,
            &user,
        )
        .unwrap();
        assert!(child.wait().unwrap().success());
        assert!(!request.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn 예약_작업은_설치_실행_파일과_일치할_때만_소유한_것으로_판정한다() {
        let xml = r#"<Task><Actions><Exec><Command>&quot;C:\Program Files\Nogirem\nogirem.exe&quot;</Command></Exec></Actions></Task>"#;
        assert!(same_path(
            Path::new(&task_command(xml).unwrap()),
            Path::new(r"C:\Program Files\Nogirem\nogirem.exe")
        ));
        assert!(!same_path(
            Path::new(&task_command(xml).unwrap()),
            Path::new(r"C:\Other\nogirem.exe")
        ));
    }

    #[test]
    fn 구버전_updater가_실행한_installer는_부모_updater를_보존한다() {
        let root = std::env::temp_dir().join(format!(
            "nogirem-legacy-updater-cleanup-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let install = root.join("install");
        let updater_root = root.join("local/NogiremUpdater/updates/job-test");
        let user = root.join("roaming/마비노기 렘 부스터");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&updater_root).unwrap();
        let updater = updater_root.join("updater.exe");
        let installer = updater_root.join("installer.exe");
        let state = root.join("installer-pid.txt");
        fs::copy(std::env::current_exe().unwrap(), &updater).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &installer).unwrap();
        let mut updater_child = Command::new(&updater)
            .args(["--ignored", "--exact", "tests::legacy_updater_fixture"])
            .env("NOGIREM_TEST_INSTALLER", &installer)
            .env("NOGIREM_TEST_STATE", &state)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let installer_pid = loop {
            if let Ok(value) = fs::read_to_string(&state) {
                break value.parse::<u32>().unwrap();
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(50));
        };
        let options = CleanupOptions {
            install_directory: install.canonicalize().unwrap(),
            caller_pid: installer_pid,
            updater_pid: 0,
            uninstall: false,
            timeout: Duration::from_secs(1),
        };
        cleanup_with_roots(
            &options,
            &install.canonicalize().unwrap(),
            &root.join("local/NogiremUpdater/updates"),
            &user,
        )
        .unwrap();
        assert!(updater_child.try_wait().unwrap().is_none());
        assert!(open_process(installer_pid, false).is_ok());
        if let Ok(handle) = open_process(installer_pid, true) {
            terminate(&OwnedProcess {
                pid: installer_pid,
                handle,
            });
        }
        stop(&mut updater_child);
        fs::remove_dir_all(root).unwrap();
    }
}
