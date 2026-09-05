#![windows_subsystem = "windows"]

use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, LPARAM, LRESULT, WAIT_TIMEOUT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetProcessAffinityMask, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, SetProcessAffinityMask,
    WaitForSingleObject,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NUMLOCK, VK_PAUSE, VK_PRINT,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetWindowThreadProcessId, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, PM_REMOVE, PeekMessageW,
    SPI_GETKEYBOARDDELAY, SetWindowsHookExW, SystemParametersInfoW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const REPEAT_HZ: u32 = 50;
const REPEAT_INTERVAL: Duration = Duration::from_millis(20);
const INJECTION_MARKER: usize = 0x4e4f_4749_5245_4d54;
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct KeySpec {
    vk_code: u32,
    scan_code: u16,
    extended: bool,
}

#[derive(Debug)]
struct ActiveKey {
    key: KeySpec,
    repeat_at: Instant,
    foreground_verified: bool,
    generation: u64,
}

#[derive(Default)]
struct TurboState {
    active: Option<ActiveKey>,
    modifiers: HashSet<u32>,
    pressed_keys: HashSet<KeySpec>,
    blocked_keys: HashSet<KeySpec>,
    generation: u64,
}

struct SharedState {
    state: Mutex<TurboState>,
    changed: Condvar,
    stopping: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    running: bool,
    pid: u32,
    repeat_hz: u32,
    game_only: bool,
    updated_at: u128,
    error: Option<String>,
}

static SHARED: OnceLock<Arc<SharedState>> = OnceLock::new();

fn is_modifier(vk_code: u32) -> bool {
    [
        VK_SHIFT,
        VK_LSHIFT,
        VK_RSHIFT,
        VK_CONTROL,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_MENU,
        VK_LMENU,
        VK_RMENU,
        VK_LWIN,
        VK_RWIN,
    ]
    .contains(&(vk_code as VIRTUAL_KEY))
}

fn is_excluded_key(vk_code: u32) -> bool {
    [
        VK_CAPITAL,
        VK_NUMLOCK,
        VK_SCROLL,
        VK_PAUSE,
        VK_PRINT,
        VK_SNAPSHOT,
    ]
    .contains(&(vk_code as VIRTUAL_KEY))
}

fn any_modifier_pressed() -> bool {
    [
        VK_LSHIFT,
        VK_RSHIFT,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_LMENU,
        VK_RMENU,
        VK_LWIN,
        VK_RWIN,
    ]
    .iter()
    .any(|key| unsafe { GetAsyncKeyState(i32::from(*key)) } < 0)
}

fn cancel_active(state: &mut TurboState) {
    if state.active.take().is_some() {
        state.generation = state.generation.wrapping_add(1);
    }
}

fn cancel_turbo(state: &mut TurboState) {
    cancel_active(state);
    state.blocked_keys.clear();
}

fn next_repeat_deadline(previous: Instant, now: Instant) -> Instant {
    let candidate = previous + REPEAT_INTERVAL;
    if candidate > now {
        candidate
    } else {
        now + REPEAT_INTERVAL
    }
}

fn keyboard_repeat_delay() -> Duration {
    let mut value = 1u32;
    let result = unsafe {
        SystemParametersInfoW(
            SPI_GETKEYBOARDDELAY,
            0,
            &mut value as *mut u32 as *mut c_void,
            0,
        )
    };
    if result == 0 {
        return Duration::from_millis(500);
    }
    Duration::from_millis((u64::from(value.min(3)) + 1) * 250)
}

fn use_all_available_cpus() -> Result<(), String> {
    let process = unsafe { GetCurrentProcess() };
    let mut process_mask = 0usize;
    let mut system_mask = 0usize;
    let read = unsafe { GetProcessAffinityMask(process, &mut process_mask, &mut system_mask) };
    if read == 0 || system_mask == 0 {
        return Err("터보 키 CPU 범위를 확인하지 못했습니다".to_owned());
    }
    let applied = unsafe { SetProcessAffinityMask(process, system_mask) };
    if applied == 0 {
        return Err("터보 키 CPU 범위를 복구하지 못했습니다".to_owned());
    }
    Ok(())
}

fn process_path(pid: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..length as usize],
    )))
}

fn is_mabinogi_path(path: &Path) -> bool {
    let executable_matches = path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("Client.exe"));
    let directory_matches = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("Mabinogi"));
    executable_matches && directory_matches
}

fn is_mabinogi_foreground() -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return false;
    }
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let Some(path) = process_path(pid) else {
        return false;
    };
    is_mabinogi_path(&path)
}

fn keyboard_input(key: KeySpec, key_up: bool) -> INPUT {
    let mut flags = if key.scan_code == 0 {
        0
    } else {
        KEYEVENTF_SCANCODE
    };
    if key.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: if key.scan_code == 0 {
                    key.vk_code as VIRTUAL_KEY
                } else {
                    0
                },
                wScan: key.scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTION_MARKER,
            },
        },
    }
}

fn send_key_pulse(key: KeySpec) {
    let inputs = [keyboard_input(key, false), keyboard_input(key, true)];
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        );
    }
}

fn press_key(shared: &SharedState, key: KeySpec) -> bool {
    let mut state = shared
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !state.pressed_keys.insert(key) {
        return state.blocked_keys.contains(&key);
    }
    if !state.modifiers.is_empty() {
        cancel_turbo(&mut state);
        shared.changed.notify_all();
        return false;
    }
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.active = Some(ActiveKey {
        key,
        repeat_at: Instant::now() + keyboard_repeat_delay(),
        foreground_verified: false,
        generation,
    });
    shared.changed.notify_all();
    false
}

fn release_key(shared: &SharedState, key: KeySpec) {
    let mut state = shared
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.pressed_keys.remove(&key);
    state.blocked_keys.remove(&key);
    if state
        .active
        .as_ref()
        .is_some_and(|active| active.key == key)
    {
        cancel_turbo(&mut state);
        shared.changed.notify_all();
    }
}

fn update_modifier(shared: &SharedState, vk_code: u32, pressed: bool) {
    let mut state = shared
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if pressed {
        state.modifiers.insert(vk_code);
        cancel_turbo(&mut state);
    } else {
        state.modifiers.remove(&vk_code);
    }
    shared.changed.notify_all();
}

unsafe extern "system" fn keyboard_hook(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
        if event.flags & LLKHF_INJECTED == 0
            && event.dwExtraInfo != INJECTION_MARKER
            && let Some(shared) = SHARED.get()
        {
            let pressed = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
            let released = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;
            if is_modifier(event.vkCode) {
                if pressed || released {
                    update_modifier(shared, event.vkCode, pressed);
                }
            } else if is_excluded_key(event.vkCode) {
                if pressed {
                    let mut state = shared
                        .state
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    cancel_turbo(&mut state);
                    shared.changed.notify_all();
                }
            } else {
                let key = KeySpec {
                    vk_code: event.vkCode,
                    scan_code: event.scanCode as u16,
                    extended: event.flags & LLKHF_EXTENDED != 0,
                };
                if pressed {
                    if press_key(shared, key) {
                        return 1;
                    }
                } else if released {
                    release_key(shared, key);
                }
            }
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, w_param, l_param) }
}

fn repeat_loop(shared: Arc<SharedState>) {
    let mut state = shared
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    while !shared.stopping.load(Ordering::Acquire) {
        let Some(active) = state.active.as_ref() else {
            state = shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
            continue;
        };
        if !active.foreground_verified {
            let key = active.key;
            let generation = active.generation;
            drop(state);
            let valid = is_mabinogi_foreground() && !any_modifier_pressed();
            state = shared
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(current) = state
                .active
                .as_mut()
                .filter(|current| current.key == key && current.generation == generation)
            {
                if valid {
                    current.foreground_verified = true;
                    state.blocked_keys.insert(key);
                } else {
                    cancel_turbo(&mut state);
                }
            }
            continue;
        }
        let now = Instant::now();
        if active.repeat_at > now {
            let duration = active.repeat_at.duration_since(now);
            let result = shared.changed.wait_timeout(state, duration);
            state = result.unwrap_or_else(|error| error.into_inner()).0;
            continue;
        }
        let key = active.key;
        let generation = active.generation;
        if !state.modifiers.is_empty() {
            cancel_turbo(&mut state);
            continue;
        }
        drop(state);
        let valid = is_mabinogi_foreground() && !any_modifier_pressed();
        state = shared
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let still_active = state
            .active
            .as_ref()
            .is_some_and(|current| current.key == key && current.generation == generation);
        if !still_active {
            continue;
        }
        if !valid {
            cancel_turbo(&mut state);
            continue;
        }
        if let Some(active) = state.active.as_mut() {
            active.repeat_at = next_repeat_deadline(active.repeat_at, now);
        }
        drop(state);
        send_key_pulse(key);
        state = shared
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn write_status(path: &Path, running: bool, error: Option<String>) -> Result<(), String> {
    let status = Status {
        running,
        pid: std::process::id(),
        repeat_hz: REPEAT_HZ,
        game_only: true,
        updated_at: now_millis(),
        error,
    };
    let contents = serde_json::to_vec(&status)
        .map_err(|error| format!("터보 키 상태를 직렬화하지 못했습니다: {error}"))?;
    let temporary = path.with_extension("tmp");
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)
            .map_err(|error| format!("터보 키 상태 폴더를 만들지 못했습니다: {error}"))?;
    }
    fs::write(&temporary, contents)
        .map_err(|error| format!("터보 키 임시 상태를 쓰지 못했습니다: {error}"))?;
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!("이전 터보 키 상태를 교체하지 못했습니다: {error}"));
        }
    }
    fs::rename(temporary, path)
        .map_err(|error| format!("터보 키 상태를 저장하지 못했습니다: {error}"))
}

fn argument(name: &str) -> Option<String> {
    env::args().find_map(|value| value.strip_prefix(&format!("--{name}=")).map(str::to_owned))
}

fn parent_is_alive(parent_pid: u32) -> bool {
    let handle = unsafe { OpenProcess(SYNCHRONIZE_ACCESS, 0, parent_pid) };
    if handle.is_null() {
        return false;
    }
    let result = unsafe { WaitForSingleObject(handle, 0) };
    unsafe {
        CloseHandle(handle);
    }
    result == WAIT_TIMEOUT
}

fn should_stop(control_path: &Path) -> bool {
    fs::read_to_string(control_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .and_then(|value| {
            value
                .get("command")
                .and_then(|command| command.as_str())
                .map(str::to_owned)
        })
        .is_some_and(|command| command == "stop")
}

fn run() -> Result<(), String> {
    let status_path = argument("status-path")
        .map(PathBuf::from)
        .ok_or_else(|| "status-path 인자가 필요합니다".to_owned())?;
    let control_path = argument("control-path")
        .map(PathBuf::from)
        .ok_or_else(|| "control-path 인자가 필요합니다".to_owned())?;
    let parent_pid = argument("parent-pid")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| "parent-pid 인자가 필요합니다".to_owned())?;
    write_status(&status_path, false, None)?;
    use_all_available_cpus()?;
    let mutex_name: Vec<u16> = "Local\\NogiremTurboKey\0".encode_utf16().collect();
    let mutex = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
    if mutex.is_null() {
        return Err("터보 키 단일 실행 잠금을 만들지 못했습니다".to_owned());
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(mutex);
        }
        return Err("터보 키가 이미 실행 중입니다".to_owned());
    }

    let shared = Arc::new(SharedState {
        state: Mutex::new(TurboState::default()),
        changed: Condvar::new(),
        stopping: AtomicBool::new(false),
    });
    SHARED
        .set(shared.clone())
        .map_err(|_| "터보 키 상태를 초기화하지 못했습니다".to_owned())?;
    let repeat_shared = shared.clone();
    let repeat_thread = thread::spawn(move || repeat_loop(repeat_shared));
    let module = unsafe { GetModuleHandleW(null()) };
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) };
    if hook.is_null() {
        shared.stopping.store(true, Ordering::Release);
        shared.changed.notify_all();
        let _ = repeat_thread.join();
        unsafe {
            CloseHandle(mutex);
        }
        return Err("키보드 입력 감시를 시작하지 못했습니다".to_owned());
    }

    let _ = fs::remove_file(&control_path);
    let mut runtime_error = write_status(&status_path, true, None).err();
    let mut message: MSG = unsafe { zeroed() };
    let mut next_status = Instant::now() + Duration::from_secs(1);
    while runtime_error.is_none() {
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if message.message == WM_QUIT {
                shared.stopping.store(true, Ordering::Release);
                break;
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if shared.stopping.load(Ordering::Acquire)
            || should_stop(&control_path)
            || !parent_is_alive(parent_pid)
        {
            break;
        }
        if Instant::now() >= next_status {
            runtime_error = write_status(&status_path, true, None).err();
            next_status = Instant::now() + Duration::from_secs(1);
        }
        thread::sleep(Duration::from_millis(5));
    }

    shared.stopping.store(true, Ordering::Release);
    {
        let mut state = shared
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.active = None;
    }
    shared.changed.notify_all();
    unsafe {
        UnhookWindowsHookEx(hook);
    }
    let _ = repeat_thread.join();
    let final_status = write_status(&status_path, false, runtime_error.clone());
    unsafe {
        CloseHandle(mutex);
    }
    if let Some(error) = runtime_error {
        return Err(error);
    }
    final_status
}

fn main() {
    if let Err(error) = run() {
        if let Some(path) = argument("status-path").map(PathBuf::from) {
            let _ = write_status(&path, false, Some(error));
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_and_lock_keys_are_excluded() {
        assert!(is_modifier(VK_SHIFT as u32));
        assert!(is_modifier(VK_RCONTROL as u32));
        assert!(is_modifier(VK_LMENU as u32));
        assert!(is_modifier(VK_RWIN as u32));
        assert!(is_excluded_key(VK_CAPITAL as u32));
        assert!(is_excluded_key(VK_NUMLOCK as u32));
        assert!(!is_excluded_key('1' as u32));
    }

    #[test]
    fn foreground_target_requires_client_in_mabinogi_directory() {
        assert!(is_mabinogi_path(Path::new(r"D:\Games\Mabinogi\Client.exe")));
        assert!(!is_mabinogi_path(Path::new(r"D:\Games\Other\Client.exe")));
        assert!(!is_mabinogi_path(Path::new(
            r"D:\Games\Mabinogi\Launcher.exe"
        )));
    }

    #[test]
    fn late_repeat_does_not_catch_up_in_a_burst() {
        assert_eq!(REPEAT_HZ, 50);
        assert_eq!(REPEAT_INTERVAL, Duration::from_millis(20));
        let previous = Instant::now();
        let late = previous + Duration::from_millis(200);
        let next = next_repeat_deadline(previous, late);
        assert!(next >= late + REPEAT_INTERVAL);
    }

    #[test]
    fn latest_key_replaces_the_previous_key() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
        };
        let first = KeySpec {
            vk_code: '1' as u32,
            scan_code: 2,
            extended: false,
        };
        let second = KeySpec {
            vk_code: '2' as u32,
            scan_code: 3,
            extended: false,
        };
        press_key(&shared, first);
        press_key(&shared, second);
        let state = shared.state.lock().unwrap();
        assert_eq!(state.active.as_ref().map(|active| active.key), Some(second));
    }

    #[test]
    fn verified_key_blocks_windows_auto_repeat() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
        };
        let key = KeySpec {
            vk_code: '1' as u32,
            scan_code: 2,
            extended: false,
        };
        assert!(!press_key(&shared, key));
        shared
            .state
            .lock()
            .unwrap()
            .active
            .as_mut()
            .unwrap()
            .foreground_verified = true;
        shared.state.lock().unwrap().blocked_keys.insert(key);
        assert!(press_key(&shared, key));
    }

    #[test]
    fn older_held_key_cannot_replace_latest_key() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
        };
        let first = KeySpec {
            vk_code: 'A' as u32,
            scan_code: 30,
            extended: false,
        };
        let second = KeySpec {
            vk_code: 'B' as u32,
            scan_code: 48,
            extended: false,
        };
        press_key(&shared, first);
        shared.state.lock().unwrap().blocked_keys.insert(first);
        press_key(&shared, second);
        assert!(press_key(&shared, first));
        let state = shared.state.lock().unwrap();
        assert_eq!(state.active.as_ref().map(|active| active.key), Some(second));
    }

    #[test]
    fn pressing_modifier_cancels_active_key() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
        };
        press_key(
            &shared,
            KeySpec {
                vk_code: 'Q' as u32,
                scan_code: 16,
                extended: false,
            },
        );
        update_modifier(&shared, VK_SHIFT as u32, true);
        let state = shared.state.lock().unwrap();
        assert!(state.active.is_none());
    }
}
