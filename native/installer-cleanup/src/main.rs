use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Globalization::CompareStringOrdinal;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const SYNCHRONIZE_ACCESS: u32 = 0x00100000;
const ERROR_INVALID_PARAMETER_CODE: u32 = 87;
const STARTUP_TASK_NAME: &str = r"\Mabinogi Rem Booster Startup";
const STARTUP_ARGUMENTS: &str = "--startup-tray";

#[derive(Debug)]
struct Options {
    install_directory: PathBuf,
    caller_pid: u32,
    updater_pid: u32,
    uninstall: bool,
    graceful_timeout: Duration,
}

#[derive(Clone, Debug)]
struct ProcessInfo {
    pid: u32,
    parent_pid: u32,
    path: PathBuf,
    creation_time: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = parse_options(env::args_os().skip(1))?;
    let install_prefixes = directory_prefixes(&options.install_directory)?;
    let target_exe = normalized_path(&options.install_directory.join("nogirem.exe"))?;
    let updater_directory: PathBuf = env::var_os("LOCALAPPDATA")
        .ok_or("LOCALAPPDATA 환경 변수를 찾지 못했습니다.")?
        .into();
    let updater_prefixes = directory_prefixes(&updater_directory.join("NogiremUpdater/updates"))?;
    let current_pid = std::process::id();
    let initial_processes = process_snapshot()?;
    let caller = initial_processes
        .iter()
        .find(|process| process.pid == options.caller_pid)
        .ok_or("설치기 프로세스를 확인하지 못했습니다.")?;
    let preserved_updater = if options.updater_pid > 0 {
        if caller.parent_pid != options.updater_pid {
            return Err("업데이트 설치 프로세스의 부모와 경로를 확인하지 못했습니다.".into());
        }
        Some(
            trusted_updater(
                &initial_processes,
                options.updater_pid,
                caller.creation_time,
                &updater_prefixes,
            )
            .cloned()
            .ok_or("업데이트 설치 프로세스의 부모와 경로를 확인하지 못했습니다.")?,
        )
    } else {
        trusted_updater(
            &initial_processes,
            caller.parent_pid,
            caller.creation_time,
            &updater_prefixes,
        )
        .cloned()
    };

    if options.uninstall {
        cleanup_startup_task(&target_exe);
    }

    stop_updaters(
        &updater_prefixes,
        current_pid,
        options.caller_pid,
        preserved_updater.as_ref(),
    )?;

    if options.updater_pid > 0 {
        return Ok(());
    }

    let request_file = installer_close_request_path()?;
    stop_installed_processes(
        &install_prefixes,
        current_pid,
        options.caller_pid,
        &request_file,
        options.graceful_timeout,
    )
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let values: Vec<OsString> = arguments.collect();
    let mut install_directory = None;
    let mut caller_pid = None;
    let mut updater_pid = 0;
    let mut uninstall = false;
    let mut graceful_timeout = 30;
    let mut updater_pid_seen = false;
    let mut graceful_timeout_seen = false;
    let mut index = 0;

    while index < values.len() {
        let name = values[index].to_string_lossy();
        match name.as_ref() {
            "--install-directory" => {
                if install_directory.is_some() {
                    return Err("--install-directory를 중복 지정할 수 없습니다.".into());
                }
                index += 1;
                install_directory = values.get(index).map(PathBuf::from);
            }
            "--caller-pid" => {
                if caller_pid.is_some() {
                    return Err("--caller-pid를 중복 지정할 수 없습니다.".into());
                }
                index += 1;
                caller_pid = Some(parse_u32(values.get(index), "--caller-pid")?);
            }
            "--updater-pid" => {
                if updater_pid_seen {
                    return Err("--updater-pid를 중복 지정할 수 없습니다.".into());
                }
                updater_pid_seen = true;
                index += 1;
                updater_pid = parse_u32(values.get(index), "--updater-pid")?;
                if updater_pid == 0 {
                    return Err("--updater-pid는 0보다 커야 합니다.".into());
                }
            }
            "--graceful-timeout-seconds" => {
                if graceful_timeout_seen {
                    return Err("--graceful-timeout-seconds를 중복 지정할 수 없습니다.".into());
                }
                graceful_timeout_seen = true;
                index += 1;
                graceful_timeout = parse_u32(values.get(index), "--graceful-timeout-seconds")?;
                if !(1..=300).contains(&graceful_timeout) {
                    return Err("--graceful-timeout-seconds는 1~300이어야 합니다.".into());
                }
            }
            "--uninstall" => {
                if uninstall {
                    return Err("--uninstall을 중복 지정할 수 없습니다.".into());
                }
                uninstall = true;
            }
            _ => return Err(format!("알 수 없는 인자입니다: {name}")),
        }
        index += 1;
    }

    let install_directory = install_directory.ok_or("--install-directory가 필요합니다.")?;
    if !install_directory.is_absolute() {
        return Err("--install-directory는 절대 경로여야 합니다.".into());
    }

    let caller_pid = caller_pid.ok_or("--caller-pid가 필요합니다.")?;
    if caller_pid == 0 {
        return Err("--caller-pid는 0보다 커야 합니다.".into());
    }

    Ok(Options {
        install_directory,
        caller_pid,
        updater_pid,
        uninstall,
        graceful_timeout: Duration::from_secs(graceful_timeout.into()),
    })
}

fn parse_u32(value: Option<&OsString>, name: &str) -> Result<u32, String> {
    value
        .ok_or_else(|| format!("{name} 값이 필요합니다."))?
        .to_string_lossy()
        .parse()
        .map_err(|_| format!("{name} 값이 올바른 프로세스 ID가 아닙니다."))
}

fn process_snapshot() -> Result<Vec<ProcessInfo>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(format!(
            "실행 중인 프로세스를 조회하지 못했습니다: {}",
            io::Error::last_os_error()
        ));
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut processes = Vec::new();
    let mut available = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while available {
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID) };
        if !handle.is_null() {
            if let Ok(process) =
                process_identity(handle, entry.th32ProcessID, entry.th32ParentProcessID)
            {
                processes.push(process);
            }
            unsafe {
                CloseHandle(handle);
            }
        }
        available = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe {
        CloseHandle(snapshot);
    }
    Ok(processes)
}

fn process_identity(handle: HANDLE, pid: u32, parent_pid: u32) -> Result<ProcessInfo, String> {
    let mut buffer = vec![0_u16; 32_768];
    let mut length = buffer.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) } == 0 {
        return Err(format!(
            "프로세스 {pid} 실행 경로를 재검증하지 못했습니다: {}",
            io::Error::last_os_error()
        ));
    }
    let mut creation: FILETIME = unsafe { std::mem::zeroed() };
    let mut exit: FILETIME = unsafe { std::mem::zeroed() };
    let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
    let mut user: FILETIME = unsafe { std::mem::zeroed() };
    if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(format!(
            "프로세스 {pid} 생성 시각을 재검증하지 못했습니다: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(ProcessInfo {
        pid,
        parent_pid,
        path: PathBuf::from(OsString::from_wide(&buffer[..length as usize])),
        creation_time: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
    })
}

fn trusted_updater<'a>(
    processes: &'a [ProcessInfo],
    pid: u32,
    caller_creation_time: u64,
    updater_prefixes: &[Vec<u16>],
) -> Option<&'a ProcessInfo> {
    processes.iter().find(|process| {
        process.pid == pid
            && process.creation_time <= caller_creation_time
            && path_is_under(&process.path, updater_prefixes)
            && process
                .path
                .file_name()
                .is_some_and(|name| windows_equal_os(name, OsStr::new("updater.exe")))
    })
}

fn same_process(left: &ProcessInfo, right: &ProcessInfo) -> bool {
    left.pid == right.pid
        && left.creation_time == right.creation_time
        && paths_equal(&left.path, &right.path)
}

fn stop_updaters(
    updater_prefixes: &[Vec<u16>],
    current_pid: u32,
    caller_pid: u32,
    preserved_updater: Option<&ProcessInfo>,
) -> Result<(), String> {
    let ignored = [current_pid, caller_pid];
    let mut running = matching_processes(updater_prefixes, &ignored, preserved_updater)?;
    if running.is_empty() {
        return Ok(());
    }

    for process in &running {
        terminate_process(process)?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
        running = matching_processes(updater_prefixes, &ignored, preserved_updater)?;
        if running.is_empty() {
            return Ok(());
        }
    }
    Err("업데이트 설치 프로세스를 종료하지 못했습니다. 작업 관리자에서 앱 업데이트를 종료한 뒤 다시 시도하세요.".into())
}

fn stop_installed_processes(
    install_prefixes: &[Vec<u16>],
    current_pid: u32,
    caller_pid: u32,
    request_file: &Path,
    graceful_timeout: Duration,
) -> Result<(), String> {
    if let Some(parent) = request_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("앱 종료 요청 폴더를 만들지 못했습니다: {error}"))?;
    }
    let ignored = [current_pid, caller_pid];
    let graceful_deadline = Instant::now() + graceful_timeout;
    let mut force_deadline = None;
    let mut quiet_since = None;
    let mut last_request = None;

    loop {
        let running = matching_processes(install_prefixes, &ignored, None)?;
        if running.is_empty() {
            let since = quiet_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(1) {
                let _ = fs::remove_file(request_file);
                return Ok(());
            }
        } else {
            quiet_since = None;
            if last_request
                .is_none_or(|instant: Instant| instant.elapsed() >= Duration::from_secs(1))
            {
                write_close_request(request_file)?;
                last_request = Some(Instant::now());
            }
            if Instant::now() >= graceful_deadline {
                for process in &running {
                    terminate_process(process)?;
                }
                let deadline =
                    force_deadline.get_or_insert_with(|| Instant::now() + Duration::from_secs(5));
                if Instant::now() >= *deadline
                    && !matching_processes(install_prefixes, &ignored, None)?.is_empty()
                {
                    return Err("설치 폴더의 앱 프로세스를 종료하지 못했습니다. 작업 관리자에서 앱을 종료한 뒤 다시 시도하세요.".into());
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn matching_processes(
    prefixes: &[Vec<u16>],
    ignored: &[u32],
    preserved: Option<&ProcessInfo>,
) -> Result<Vec<ProcessInfo>, String> {
    Ok(process_snapshot()?
        .into_iter()
        .filter(|process| {
            !ignored.contains(&process.pid)
                && !preserved.is_some_and(|expected| same_process(process, expected))
                && path_is_under(&process.path, prefixes)
        })
        .collect())
}

fn terminate_process(expected: &ProcessInfo) -> Result<(), String> {
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZE_ACCESS,
            0,
            expected.pid,
        )
    };
    if handle.is_null() {
        let code = unsafe { GetLastError() };
        if code == ERROR_INVALID_PARAMETER_CODE {
            return Ok(());
        }
        return Err(format!(
            "프로세스 {} 종료 권한을 얻지 못했습니다: {}",
            expected.pid,
            io::Error::from_raw_os_error(code as i32)
        ));
    }
    let current = match process_identity(handle, expected.pid, expected.parent_pid) {
        Ok(process) => process,
        Err(error) => {
            let exited = unsafe { WaitForSingleObject(handle, 0) } == 0;
            unsafe {
                CloseHandle(handle);
            }
            if exited {
                return Ok(());
            }
            return Err(error);
        }
    };
    if !same_process(&current, expected) {
        unsafe {
            CloseHandle(handle);
        }
        return Ok(());
    }
    if unsafe { TerminateProcess(handle, 1) } == 0 {
        let exited = unsafe { WaitForSingleObject(handle, 0) } == 0;
        let error = io::Error::last_os_error();
        unsafe {
            CloseHandle(handle);
        }
        if exited {
            return Ok(());
        }
        return Err(format!(
            "프로세스 {} 종료 요청에 실패했습니다: {error}",
            expected.pid
        ));
    }
    unsafe {
        CloseHandle(handle);
    }
    Ok(())
}

fn write_close_request(path: &Path) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    fs::write(
        path,
        format!("installer-upgrade:{}-{timestamp}", std::process::id()),
    )
    .map_err(|error| format!("앱 종료 요청을 기록하지 못했습니다: {error}"))
}

fn installer_close_request_path() -> Result<PathBuf, String> {
    let roaming = env::var_os("APPDATA").ok_or("APPDATA 환경 변수를 찾지 못했습니다.")?;
    Ok(PathBuf::from(roaming)
        .join("마비노기 렘 부스터")
        .join("instance")
        .join("installer-close-request"))
}

fn cleanup_startup_task(target_exe: &[u16]) {
    if let Err(error) = cleanup_owned_startup_task(target_exe) {
        eprintln!("시작 예약 작업을 정리하지 못했지만 앱 제거는 계속합니다: {error}");
    }
}

fn cleanup_owned_startup_task(target_exe: &[u16]) -> Result<(), String> {
    let query = run_schtasks(&["/Query", "/TN", STARTUP_TASK_NAME, "/XML"])?;
    if !query.status.success() {
        return Ok(());
    }
    let xml = decode_output(&query.stdout);
    if !task_xml_is_owned(&xml, target_exe) {
        return Ok(());
    }
    let deletion = run_schtasks(&["/Delete", "/TN", STARTUP_TASK_NAME, "/F"])?;
    if deletion.status.success() {
        Ok(())
    } else {
        Err(output_error(&deletion))
    }
}

fn run_schtasks(arguments: &[&str]) -> Result<Output, String> {
    let system_root = env::var_os("SystemRoot").ok_or("SystemRoot 환경 변수를 찾지 못했습니다.")?;
    Command::new(PathBuf::from(system_root).join("System32/schtasks.exe"))
        .args(arguments)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("예약 작업 명령을 실행하지 못했습니다: {error}"))
}

fn output_error(output: &Output) -> String {
    let stderr = decode_output(&output.stderr);
    let stdout = decode_output(&output.stdout);
    let detail = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("원인을 확인하지 못했습니다.");
    format!("종료 코드 {:?}: {detail}", output.status.code())
}

fn decode_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        let units = (2..bytes.len().saturating_sub(1))
            .step_by(2)
            .map(|index| u16::from_le_bytes([bytes[index], bytes[index + 1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn task_xml_is_owned(xml: &str, target_exe: &[u16]) -> bool {
    if xml.match_indices("<Exec>").count() != 1 {
        return false;
    }
    let Some(command) = xml_value(xml, "Command") else {
        return false;
    };
    let Some(arguments) = xml_value(xml, "Arguments") else {
        return false;
    };
    let Ok(command) = normalized_path(Path::new(command.trim_matches('"'))) else {
        return false;
    };
    windows_equal(&command, target_exe) && arguments == STARTUP_ARGUMENTS
}

fn xml_value(xml: &str, name: &str) -> Option<String> {
    let start_tag = format!("<{name}>");
    let end_tag = format!("</{name}>");
    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;
    Some(
        xml[start..end]
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )
}

fn directory_prefixes(path: &Path) -> Result<Vec<Vec<u16>>, String> {
    let mut paths = vec![path.to_path_buf()];
    if let Ok(canonical) = fs::canonicalize(path) {
        paths.push(canonical);
    }
    let mut prefixes: Vec<Vec<u16>> = Vec::new();
    for candidate in paths {
        let mut value = normalized_path(&candidate)?;
        if !value.ends_with(&[b'\\' as u16]) {
            value.push(b'\\' as u16);
        }
        if !prefixes
            .iter()
            .any(|existing| windows_equal(existing, &value))
        {
            prefixes.push(value);
        }
    }
    Ok(prefixes)
}

fn normalized_path(path: &Path) -> Result<Vec<u16>, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("현재 폴더를 확인하지 못했습니다: {error}"))?
            .join(path)
    };
    let mut value = absolute.as_os_str().encode_wide().collect::<Vec<_>>();
    for unit in &mut value {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    let extended_prefix = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    if value.starts_with(&extended_prefix) {
        value.drain(..extended_prefix.len());
    }
    while value.last() == Some(&(b'\\' as u16)) {
        value.pop();
    }
    Ok(value)
}

fn path_is_under(path: &Path, prefixes: &[Vec<u16>]) -> bool {
    normalized_path(path).is_ok_and(|value| {
        prefixes.iter().any(|prefix| {
            value.len() > prefix.len()
                && value
                    .get(..prefix.len())
                    .is_some_and(|candidate| windows_equal(candidate, prefix))
        })
    })
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    normalized_path(left)
        .ok()
        .zip(normalized_path(right).ok())
        .is_some_and(|(left, right)| windows_equal(&left, &right))
}

fn windows_equal_os(left: &OsStr, right: &OsStr) -> bool {
    let left = left.encode_wide().collect::<Vec<_>>();
    let right = right.encode_wide().collect::<Vec<_>>();
    windows_equal(&left, &right)
}

fn windows_equal(left: &[u16], right: &[u16]) -> bool {
    if left.is_empty() || right.is_empty() {
        return left == right;
    }
    unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        ) == 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_owned_startup_task_matches() {
        let target = r"C:\Program Files\Nogirem\nogirem.exe";
        let target_path = normalized_path(Path::new(target)).unwrap();
        let owned = format!(
            "<Task><Actions><Exec><Command>{target}</Command><Arguments>--startup-tray</Arguments></Exec></Actions></Task>"
        );
        assert!(task_xml_is_owned(&owned, &target_path));
        assert!(!task_xml_is_owned(
            &owned.replace("--startup-tray", "--other"),
            &target_path
        ));
        assert!(!task_xml_is_owned(
            &owned.replace("</Actions>", &format!("<Exec><Command>{target}</Command><Arguments>--startup-tray</Arguments></Exec></Actions>")),
            &target_path
        ));
    }

    #[test]
    fn path_scope_requires_directory_boundary() {
        let prefixes = directory_prefixes(Path::new(r"C:\Program Files\Nogirem")).unwrap();
        assert!(path_is_under(
            Path::new(r"C:\Program Files\Nogirem\nogirem.exe"),
            &prefixes
        ));
        assert!(!path_is_under(
            Path::new(r"C:\Program Files\Nogirem-old\nogirem.exe"),
            &prefixes
        ));
    }

    #[test]
    fn xml_entities_are_decoded_before_path_comparison() {
        let xml = r"<Exec><Command>C:\A&amp;B\nogirem.exe</Command><Arguments>--startup-tray</Arguments></Exec>";
        let target = normalized_path(Path::new(r"C:\A&B\nogirem.exe")).unwrap();
        assert!(task_xml_is_owned(xml, &target));
    }

    #[test]
    fn duplicate_options_are_rejected() {
        let arguments = [
            "--install-directory",
            r"C:\Program Files\Nogirem",
            "--caller-pid",
            "100",
            "--updater-pid",
            "200",
            "--install-directory",
            r"C:\",
        ]
        .into_iter()
        .map(OsString::from);
        assert!(parse_options(arguments).is_err());
    }

    #[test]
    fn windows_unicode_comparison_is_case_insensitive() {
        assert!(windows_equal_os(
            OsStr::new(r"C:\Nogirem\ÉXAMPLE.exe"),
            OsStr::new(r"c:\nogirem\éxample.EXE")
        ));
    }

    #[test]
    fn reused_parent_pid_is_not_trusted() {
        let updater = ProcessInfo {
            pid: 200,
            parent_pid: 0,
            path: PathBuf::from(
                r"C:\Users\Test\AppData\Local\NogiremUpdater\updates\job\updater.exe",
            ),
            creation_time: 300,
        };
        let prefixes = directory_prefixes(Path::new(
            r"C:\Users\Test\AppData\Local\NogiremUpdater\updates",
        ))
        .unwrap();
        assert!(trusted_updater(&[updater], 200, 250, &prefixes).is_none());
    }
}
