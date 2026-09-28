use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        RemoteDesktop::ProcessIdToSessionId,
        Threading::{
            GetProcessAffinityMask, GetProcessTimes, OpenProcess,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_SYNCHRONIZE,
            QueryFullProcessImageNameW, SetProcessAffinityMask, WaitForSingleObject,
        },
    },
};
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn open(pid: u32, writable: bool) -> Result<Handle> {
    let access =
        PROCESS_QUERY_LIMITED_INFORMATION | if writable { PROCESS_SET_INFORMATION } else { 0 };
    let h = unsafe { OpenProcess(access, 0, pid) };
    if h.is_null() {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(Handle(h))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub start_time: Option<String>,
    pub session_id: Option<u32>,
}
pub fn path(pid: u32) -> Result<String> {
    let h = open(pid, false)?;
    let mut data = vec![0u16; 32768];
    let mut len = data.len() as u32;
    if unsafe { QueryFullProcessImageNameW(h.0, 0, data.as_mut_ptr(), &mut len) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(String::from_utf16_lossy(&data[..len as usize]))
}
pub fn affinity(pid: u32) -> Result<u64> {
    let h = open(pid, false)?;
    let (mut process, mut system) = (0, 0);
    if unsafe { GetProcessAffinityMask(h.0, &mut process, &mut system) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(process as u64)
}
pub fn set_affinity(pid: u32, mask: u64) -> Result<()> {
    if mask == 0 {
        return Err("CPU affinity mask cannot be zero".into());
    }
    let h = open(pid, true)?;
    if unsafe { SetProcessAffinityMask(h.0, mask as usize) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
pub fn start_ms(pid: u32) -> Result<i64> {
    let h = open(pid, false)?;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(h.0, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let ticks = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
    Ok(((ticks as i128 - 116444736000000000) / 10000) as i64)
}
pub fn start_time(pid: u32) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(start_ms(pid).ok()?)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}
pub fn session(pid: u32) -> Option<u32> {
    let mut id = 0;
    (unsafe { ProcessIdToSessionId(pid, &mut id) } != 0).then_some(id)
}
pub fn alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if h.is_null() {
        return false;
    }
    let h = Handle(h);
    unsafe { WaitForSingleObject(h.0, 0) == 258 }
}
pub fn list(with_details: bool) -> Result<Vec<Process>> {
    let h = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if h == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let h = Handle(h);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut result = Vec::new();
    let mut present = unsafe { Process32FirstW(h.0, &mut entry) };
    while present != 0 {
        let name = String::from_utf16_lossy(
            &entry.szExeFile[..entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len())],
        );
        if !name.is_empty() {
            let pid = entry.th32ProcessID;
            result.push(Process {
                pid,
                name,
                path: if with_details { path(pid).ok() } else { None },
                start_time: if with_details { start_time(pid) } else { None },
                session_id: if with_details { session(pid) } else { None },
            });
        }
        present = unsafe { Process32NextW(h.0, &mut entry) };
    }
    Ok(result)
}
fn normalized(value: &str) -> String {
    value.replace('/', "\\").to_lowercase()
}
pub fn matches_game(info: &Process, config: &serde_json::Value) -> bool {
    let Some(path) = info.path.as_deref() else {
        return false;
    };
    let executable = config["gameExecutableName"].as_str().unwrap_or_else(|| {
        config["gameExecutable"]
            .as_str()
            .unwrap_or("Client.exe")
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or("Client.exe")
    });
    if !info.name.eq_ignore_ascii_case(executable) {
        return false;
    }
    let p = normalized(path);
    if p == normalized(config["gameExecutable"].as_str().unwrap_or("")) {
        return true;
    }
    let parent = p.rsplit('\\').nth(1).unwrap_or("");
    let default = vec![serde_json::json!(
        config["gameDirectoryName"].as_str().unwrap_or("Mabinogi")
    )];
    let names = config["gameDirectoryNames"].as_array().unwrap_or(&default);
    if names
        .iter()
        .any(|n| n.as_str().unwrap_or("").trim().to_lowercase() == parent)
    {
        return true;
    }
    Path::new(path)
        .parent()
        .is_some_and(|d| d.join("Mabinogi.exe").is_file())
}
// Keep a handle to the verified process, rather than enumerating every process
// on each memory-cleaner tick. A handle remains tied to the original process
// after exit, so a reused PID cannot be mistaken for the game.
#[derive(Default)]
pub struct GameWatch {
    process: Option<Handle>,
}
impl GameWatch {
    pub fn active(&mut self, config: &serde_json::Value) -> Result<bool> {
        if self.process.as_ref().is_some_and(|h| unsafe { WaitForSingleObject(h.0, 0) == 258 }) {
            return Ok(true);
        }
        self.process = None;
        let name = config["gameExecutableName"].as_str().unwrap_or("Client.exe");
        for mut p in list(false)? {
            if !p.name.eq_ignore_ascii_case(name) { continue; }
            let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, 0, p.pid) };
            if h.is_null() { continue; }
            let h = Handle(h);
            let mut data = vec![0u16; 32768];
            let mut len = data.len() as u32;
            if unsafe { QueryFullProcessImageNameW(h.0, 0, data.as_mut_ptr(), &mut len) } == 0 { continue; }
            p.path = Some(String::from_utf16_lossy(&data[..len as usize]));
            if matches_game(&p, config) && unsafe { WaitForSingleObject(h.0, 0) == 258 } {
                self.process = Some(h);
                return Ok(true);
            }
        }
        Ok(false)
    }
}
pub fn game_active(config: &serde_json::Value) -> Result<bool> {
    let name = config["gameExecutableName"]
        .as_str()
        .unwrap_or("Client.exe");
    for mut p in list(false)? {
        if p.name.eq_ignore_ascii_case(name) {
            p.path = path(p.pid).ok();
            if matches_game(&p, config) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_watch_matches_paths_and_tracks_a_live_process() {
        let exe=std::env::current_exe().unwrap();
        let config=serde_json::json!({"gameExecutableName":exe.file_name().unwrap().to_str().unwrap(),"gameExecutable":exe.to_str().unwrap()});
        let mut watch=GameWatch::default();
        assert!(watch.active(&config).unwrap());
        let handle=watch.process.as_ref().unwrap().0;
        assert!(watch.active(&config).unwrap());
        assert_eq!(watch.process.as_ref().unwrap().0,handle);
        assert!(!GameWatch::default().active(&serde_json::json!({"gameExecutableName":"nogirem-nonexistent-test.exe"})).unwrap());
    }
    #[test]
    fn own_process_queries_are_read_only() {
        let pid = std::process::id();
        assert!(list(false).unwrap().iter().any(|p| p.pid == pid));
        assert!(path(pid).unwrap().to_lowercase().ends_with(".exe"));
        assert!(start_ms(pid).unwrap() > 0);
        assert!(affinity(pid).unwrap() > 0);
        assert!(alive(pid));
    }
    #[test]
    fn game_identification_excludes_other_clients() {
        let config = serde_json::json!({"gameExecutableName":"Client.exe","gameDirectoryNames":["Mabinogi","마비노기"]});
        let mut p = Process {
            pid: 1,
            name: "Client.exe".into(),
            path: Some("D:\\마비노기\\Client.exe".into()),
            start_time: None,
            session_id: None,
        };
        assert!(matches_game(&p, &config));
        p.path = Some("D:\\OtherGame\\Client.exe".into());
        assert!(!matches_game(&p, &config));
    }
}
