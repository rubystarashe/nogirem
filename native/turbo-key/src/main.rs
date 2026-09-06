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
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, LPARAM, LRESULT, WAIT_OBJECT_0, WAIT_TIMEOUT,
    WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateMutexW,
    CreateWaitableTimerExW, GetCurrentProcess, GetCurrentThread, GetProcessAffinityMask, INFINITE,
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, SetPriorityClass,
    SetProcessAffinityMask, SetThreadIdealProcessor, SetThreadPriority, SetWaitableTimerEx,
    THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_HIGHEST, TIMER_ALL_ACCESS, WaitForSingleObject,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NUMLOCK, VK_PAUSE, VK_PRINT,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetWindowThreadProcessId, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, MWMO_INPUTAVAILABLE,
    MsgWaitForMultipleObjectsEx, PM_REMOVE, PeekMessageW, QS_ALLINPUT, SPI_GETKEYBOARDDELAY,
    SetWindowsHookExW, SystemParametersInfoW, TranslateMessage, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const DEFAULT_REPEAT_INTERVAL_MS: u64 = 1;
const REPEAT_INTERVAL_OPTIONS_MS: [u64; 6] = [1, 3, 5, 10, 20, 30];
const INJECTION_MARKER: usize = 0x4e4f_4749_5245_4d54;
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;
const HEALTH_CHECK_INTERVAL_MS: u32 = 250;

struct PrecisionTimer {
    handle: *mut c_void,
}

impl PrecisionTimer {
    fn new() -> Option<Self> {
        let high_resolution = unsafe {
            CreateWaitableTimerExW(
                null(),
                null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS,
            )
        };
        let handle = if high_resolution.is_null() {
            unsafe { CreateWaitableTimerExW(null(), null(), 0, TIMER_ALL_ACCESS) }
        } else {
            high_resolution
        };
        (!handle.is_null()).then_some(Self { handle })
    }

    fn wait(&self, duration: Duration) -> bool {
        let ticks = duration.as_nanos().div_ceil(100).clamp(1, i64::MAX as u128) as i64;
        let due_time = -ticks;
        let armed =
            unsafe { SetWaitableTimerEx(self.handle, &due_time, 0, None, null(), null(), 0) };
        armed != 0 && unsafe { WaitForSingleObject(self.handle, INFINITE) } == WAIT_OBJECT_0
    }
}

impl Drop for PrecisionTimer {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

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
    foreground_pid: Option<u32>,
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
    allowed_keys: HashSet<u32>,
    repeat_interval: Duration,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    running: bool,
    pid: u32,
    repeat_hz: u32,
    interval_ms: u64,
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

fn next_repeat_deadline(previous: Instant, now: Instant, repeat_interval: Duration) -> Instant {
    let candidate = previous + repeat_interval;
    if candidate > now {
        candidate
    } else {
        now + repeat_interval
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

fn parse_affinity_mask(value: &str) -> Result<usize, String> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    let mask = usize::from_str_radix(digits, 16)
        .map_err(|_| "터보 키 CPU 마스크가 올바르지 않습니다".to_owned())?;
    if mask == 0 {
        return Err("터보 키 CPU 마스크는 0일 수 없습니다".to_owned());
    }
    Ok(mask)
}

fn parse_selected_keys(value: &str) -> Result<HashSet<u32>, String> {
    if value.is_empty() {
        return Ok(HashSet::new());
    }
    value
        .split(',')
        .map(|item| {
            let key = item
                .parse::<u32>()
                .map_err(|_| "터보 키 선택 목록이 올바르지 않습니다".to_owned())?;
            if key == 0 || key > u8::MAX.into() || is_modifier(key) || is_excluded_key(key) {
                return Err("터보 키 선택 목록에 지원하지 않는 키가 있습니다".to_owned());
            }
            Ok(key)
        })
        .collect()
}

fn parse_repeat_interval_ms(value: &str) -> Result<u64, String> {
    let interval = value
        .parse::<u64>()
        .map_err(|_| "터보 키 입력 간격이 올바르지 않습니다".to_owned())?;
    if !REPEAT_INTERVAL_OPTIONS_MS.contains(&interval) {
        return Err("지원하지 않는 터보 키 입력 간격입니다".to_owned());
    }
    Ok(interval)
}

fn ideal_processor_from_mask(mask: usize) -> u32 {
    mask.trailing_zeros()
}

fn apply_process_priority() -> Result<(), String> {
    let applied = unsafe { SetPriorityClass(GetCurrentProcess(), ABOVE_NORMAL_PRIORITY_CLASS) };
    if applied == 0 {
        return Err("터보 키 프로세스 우선도를 설정하지 못했습니다".to_owned());
    }
    Ok(())
}

fn apply_current_thread_priority(
    ideal_processor: Option<u32>,
    priority: i32,
) -> Result<(), String> {
    let thread = unsafe { GetCurrentThread() };
    let prioritized = unsafe { SetThreadPriority(thread, priority) };
    if prioritized == 0 {
        return Err("터보 키 스레드 우선도를 설정하지 못했습니다".to_owned());
    }
    if let Some(processor) = ideal_processor {
        let previous = unsafe { SetThreadIdealProcessor(thread, processor) };
        if previous == u32::MAX {
            return Err("터보 키 스레드 선호 코어를 설정하지 못했습니다".to_owned());
        }
    }
    Ok(())
}

fn apply_cpu_affinity(mask: usize) -> Result<(), String> {
    let process = unsafe { GetCurrentProcess() };
    let mut current_mask = 0usize;
    let mut system_mask = 0usize;
    let read = unsafe { GetProcessAffinityMask(process, &mut current_mask, &mut system_mask) };
    if read == 0 || system_mask == 0 || mask & system_mask != mask {
        return Err("터보 키 CPU 범위를 확인하지 못했습니다".to_owned());
    }
    let applied = unsafe { SetProcessAffinityMask(process, mask) };
    if applied == 0 {
        return Err("터보 키 CPU 범위를 설정하지 못했습니다".to_owned());
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
        .is_some_and(|value| {
            matches!(
                value.to_lowercase().as_str(),
                "mabinogi" | "마비노기" | "nexon"
            )
        });
    executable_matches && directory_matches
}

fn foreground_process_id() -> Option<u32> {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    (pid != 0).then_some(pid)
}

fn foreground_mabinogi_pid() -> Option<u32> {
    let pid = foreground_process_id()?;
    is_mabinogi_path(&process_path(pid)?).then_some(pid)
}

fn is_process_foreground(pid: u32) -> bool {
    foreground_process_id() == Some(pid)
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
        foreground_pid: None,
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
            } else if !shared.allowed_keys.contains(&event.vkCode) {
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

fn repeat_loop(shared: Arc<SharedState>, ideal_processor: u32) {
    let _ = apply_current_thread_priority(Some(ideal_processor), THREAD_PRIORITY_HIGHEST);
    let precision_timer = PrecisionTimer::new();
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
        if active.foreground_pid.is_none() {
            let key = active.key;
            let generation = active.generation;
            drop(state);
            let foreground_pid = if any_modifier_pressed() {
                None
            } else {
                foreground_mabinogi_pid()
            };
            state = shared
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(current) = state
                .active
                .as_mut()
                .filter(|current| current.key == key && current.generation == generation)
            {
                if let Some(pid) = foreground_pid {
                    current.foreground_pid = Some(pid);
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
            if let Some(timer) = precision_timer.as_ref() {
                drop(state);
                if !timer.wait(duration) {
                    thread::sleep(duration);
                }
                state = shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
            } else {
                let result = shared.changed.wait_timeout(state, duration);
                state = result.unwrap_or_else(|error| error.into_inner()).0;
            }
            continue;
        }
        let key = active.key;
        let generation = active.generation;
        let foreground_pid = active.foreground_pid.unwrap_or_default();
        if !state.modifiers.is_empty() {
            cancel_turbo(&mut state);
            continue;
        }
        drop(state);
        let valid = is_process_foreground(foreground_pid) && !any_modifier_pressed();
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
            active.repeat_at = next_repeat_deadline(active.repeat_at, now, shared.repeat_interval);
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

fn write_status(
    path: &Path,
    running: bool,
    interval_ms: u64,
    error: Option<String>,
) -> Result<(), String> {
    let status = Status {
        running,
        pid: std::process::id(),
        repeat_hz: (1000 / interval_ms) as u32,
        interval_ms,
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
    let affinity_mask = argument("affinity-mask")
        .ok_or_else(|| "affinity-mask 인자가 필요합니다".to_owned())
        .and_then(|value| parse_affinity_mask(&value))?;
    let selected_keys = argument("keys")
        .ok_or_else(|| "keys 인자가 필요합니다".to_owned())
        .and_then(|value| parse_selected_keys(&value))?;
    let interval_ms = argument("interval-ms")
        .ok_or_else(|| "interval-ms 인자가 필요합니다".to_owned())
        .and_then(|value| parse_repeat_interval_ms(&value))?;
    write_status(&status_path, false, interval_ms, None)?;
    apply_cpu_affinity(affinity_mask)?;
    apply_process_priority()?;
    apply_current_thread_priority(None, THREAD_PRIORITY_ABOVE_NORMAL)?;
    let ideal_processor = ideal_processor_from_mask(affinity_mask);
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
        allowed_keys: selected_keys,
        repeat_interval: Duration::from_millis(interval_ms),
    });
    SHARED
        .set(shared.clone())
        .map_err(|_| "터보 키 상태를 초기화하지 못했습니다".to_owned())?;
    let repeat_shared = shared.clone();
    let repeat_thread = thread::spawn(move || repeat_loop(repeat_shared, ideal_processor));
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
    let mut runtime_error = write_status(&status_path, true, interval_ms, None).err();
    let mut message: MSG = unsafe { zeroed() };
    let mut next_health_check = Instant::now();
    let mut next_status = Instant::now() + Duration::from_secs(2);
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
        if shared.stopping.load(Ordering::Acquire) {
            break;
        }
        let now = Instant::now();
        if now >= next_health_check {
            if should_stop(&control_path) || !parent_is_alive(parent_pid) {
                break;
            }
            next_health_check = now + Duration::from_millis(HEALTH_CHECK_INTERVAL_MS as u64);
        }
        if now >= next_status {
            runtime_error = write_status(&status_path, true, interval_ms, None).err();
            next_status = now + Duration::from_secs(2);
        }
        unsafe {
            MsgWaitForMultipleObjectsEx(
                0,
                null(),
                HEALTH_CHECK_INTERVAL_MS,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            );
        }
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
    let final_status = write_status(&status_path, false, interval_ms, runtime_error.clone());
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
            let interval_ms = argument("interval-ms")
                .and_then(|value| parse_repeat_interval_ms(&value).ok())
                .unwrap_or(DEFAULT_REPEAT_INTERVAL_MS);
            let _ = write_status(&path, false, interval_ms, Some(error));
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
    fn foreground_target_requires_client_in_supported_directory() {
        assert!(is_mabinogi_path(Path::new(r"D:\Games\Mabinogi\Client.exe")));
        assert!(is_mabinogi_path(Path::new(r"D:\마비노기\Client.exe")));
        assert!(is_mabinogi_path(Path::new(r"D:\Nexon\Client.exe")));
        assert!(!is_mabinogi_path(Path::new(r"D:\Games\Other\Client.exe")));
        assert!(!is_mabinogi_path(Path::new(
            r"D:\Games\Mabinogi\Launcher.exe"
        )));
    }

    #[test]
    fn late_repeat_does_not_catch_up_in_a_burst() {
        let repeat_interval = Duration::from_millis(DEFAULT_REPEAT_INTERVAL_MS);
        let previous = Instant::now();
        let late = previous + Duration::from_millis(200);
        let next = next_repeat_deadline(previous, late, repeat_interval);
        assert!(next >= late + repeat_interval);
    }

    #[test]
    fn affinity_mask_accepts_hex_and_rejects_zero() {
        assert_eq!(parse_affinity_mask("0xff").unwrap(), 0xff);
        assert_eq!(parse_affinity_mask("FF").unwrap(), 0xff);
        assert!(parse_affinity_mask("0").is_err());
        assert!(parse_affinity_mask("invalid").is_err());
    }

    #[test]
    fn first_cpu_in_affinity_mask_is_the_ideal_processor() {
        assert_eq!(ideal_processor_from_mask(0b1111_0000), 4);
        assert_eq!(ideal_processor_from_mask(0b0100_0000), 6);
    }

    #[test]
    fn selected_keys_are_parsed_and_modifiers_are_rejected() {
        assert_eq!(parse_selected_keys("49,112,123").unwrap().len(), 3);
        assert!(parse_selected_keys("").unwrap().is_empty());
        assert!(parse_selected_keys("16").is_err());
        assert!(parse_selected_keys("invalid").is_err());
    }

    #[test]
    fn repeat_interval_accepts_only_supported_options() {
        for interval in REPEAT_INTERVAL_OPTIONS_MS {
            assert_eq!(
                parse_repeat_interval_ms(&interval.to_string()).unwrap(),
                interval
            );
        }
        assert!(parse_repeat_interval_ms("2").is_err());
        assert!(parse_repeat_interval_ms("invalid").is_err());
    }

    #[test]
    fn latest_key_replaces_the_previous_key() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
            allowed_keys: HashSet::new(),
            repeat_interval: Duration::from_millis(DEFAULT_REPEAT_INTERVAL_MS),
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
            allowed_keys: HashSet::new(),
            repeat_interval: Duration::from_millis(DEFAULT_REPEAT_INTERVAL_MS),
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
            .foreground_pid = Some(1234);
        shared.state.lock().unwrap().blocked_keys.insert(key);
        assert!(press_key(&shared, key));
    }

    #[test]
    fn older_held_key_cannot_replace_latest_key() {
        let shared = SharedState {
            state: Mutex::new(TurboState::default()),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
            allowed_keys: HashSet::new(),
            repeat_interval: Duration::from_millis(DEFAULT_REPEAT_INTERVAL_MS),
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
            allowed_keys: HashSet::new(),
            repeat_interval: Duration::from_millis(DEFAULT_REPEAT_INTERVAL_MS),
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
