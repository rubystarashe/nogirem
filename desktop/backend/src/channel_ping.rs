use crate::{Result, storage, workers::Environment};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::OpenOptions,
    io::Write,
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const REMOTE_URL: &str = "https://raw.githubusercontent.com/rubystarashe/nogirem/main/channel.csv";
const MAX_CSV_BYTES: usize = 64 * 1024;
const SAMPLE_LIMIT: usize = 5;
const MEASURE_INTERVAL: Duration = Duration::from_secs(60);
const FAILURE_SYNC_INTERVAL: Duration = Duration::from_secs(60 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(900);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Endpoint {
    channel: u16,
    address: Option<SocketAddr>,
}

#[derive(Clone, Debug)]
struct Measurement {
    endpoint: Endpoint,
    samples: VecDeque<u32>,
    failed: bool,
    measured_at: Option<u64>,
}

#[derive(Debug)]
struct Runtime {
    channels: Vec<Measurement>,
    source: String,
    synced_at: Option<u64>,
    sync_error: Option<String>,
    input_error: Option<String>,
}

pub struct ChannelPing {
    directory: PathBuf,
    enabled: AtomicBool,
    runtime: RwLock<Runtime>,
    operation: Mutex<()>,
}

impl ChannelPing {
    pub fn new(env: &Environment) -> Self {
        let directory = env.user.join("channel-ping");
        let cache = directory.join("channel.csv");
        let bundled = env.root.join("channel.csv");
        let (channels, source, sync_error) = load_initial(&cache, &bundled);
        let enabled = storage::read_json(&directory.join("setting.json"))
            .ok()
            .flatten()
            .and_then(|value| value["enabled"].as_bool())
            .unwrap_or(false);
        Self {
            directory,
            enabled: AtomicBool::new(enabled),
            runtime: RwLock::new(Runtime {
                channels: measurements(channels, &[]),
                source,
                synced_at: None,
                sync_error,
                input_error: None,
            }),
            operation: Mutex::new(()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<Value> {
        storage::write_json(
            &self.directory.join("setting.json"),
            &json!({"enabled":enabled}),
        )?;
        self.enabled.store(enabled, Ordering::SeqCst);
        Ok(self.status())
    }

    pub fn status(&self) -> Value {
        let runtime = self.runtime.read().unwrap();
        json!({
            "enabled": self.enabled(),
            "source": runtime.source,
            "syncedAt": runtime.synced_at,
            "syncError": runtime.sync_error,
            "inputError": runtime.input_error,
            "measurementIntervalSeconds": MEASURE_INTERVAL.as_secs(),
            "channels": runtime.channels.iter().map(|entry| {
                let average = (!entry.samples.is_empty()).then(|| {
                    entry.samples.iter().map(|value| u64::from(*value)).sum::<u64>()
                        / entry.samples.len() as u64
                });
                json!({
                    "channel": entry.endpoint.channel,
                    "endpoint": entry.endpoint.address.map(|address| address.to_string()),
                    "averageMs": average,
                    "sampleCount": entry.samples.len(),
                    "failed": entry.failed,
                    "measuredAt": entry.measured_at
                })
            }).collect::<Vec<_>>()
        })
    }

    pub fn set_input_error(&self, error: Option<String>) {
        self.runtime.write().unwrap().input_error = error;
    }

    pub fn run(&self, stopped: impl Fn() -> bool) {
        if let Err(error) = self.refresh_remote() {
            self.runtime.write().unwrap().sync_error = Some(error);
        }
        let mut next_measurement = Instant::now();
        let mut last_failure_sync = Some(Instant::now());
        while !stopped() {
            if self.enabled() && Instant::now() >= next_measurement {
                let failed = self.measure();
                next_measurement = Instant::now() + MEASURE_INTERVAL;
                if failed
                    && last_failure_sync.is_none_or(|last| last.elapsed() >= FAILURE_SYNC_INTERVAL)
                {
                    last_failure_sync = Some(Instant::now());
                    if let Err(error) = self.refresh_remote() {
                        self.runtime.write().unwrap().sync_error = Some(error);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn refresh_remote(&self) -> Result<()> {
        let _operation = self.operation.lock().unwrap();
        let response = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| error.to_string())?
            .get(REMOTE_URL)
            .header("User-Agent", "nogirem-channel-ping")
            .send()
            .map_err(|error| format!("채널 정보 다운로드 실패: {error}"))?
            .error_for_status()
            .map_err(|error| format!("채널 정보 응답 오류: {error}"))?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CSV_BYTES as u64)
        {
            return Err("채널 정보 파일이 허용 크기를 초과했습니다".into());
        }
        let bytes = response
            .bytes()
            .map_err(|error| format!("채널 정보 읽기 실패: {error}"))?;
        if bytes.len() > MAX_CSV_BYTES {
            return Err("채널 정보 파일이 허용 크기를 초과했습니다".into());
        }
        let text =
            std::str::from_utf8(&bytes).map_err(|_| "채널 정보 파일이 UTF-8 형식이 아닙니다")?;
        let endpoints = parse_csv(text)?;
        write_atomic(&self.directory.join("channel.csv"), text.as_bytes())?;
        let mut runtime = self.runtime.write().unwrap();
        runtime.channels = measurements(endpoints, &runtime.channels);
        runtime.source = "remote".into();
        runtime.synced_at = Some(crate::now_ms());
        runtime.sync_error = None;
        Ok(())
    }

    fn measure(&self) -> bool {
        let endpoints = self
            .runtime
            .read()
            .unwrap()
            .channels
            .iter()
            .map(|entry| entry.endpoint.clone())
            .collect::<Vec<_>>();
        let results = std::thread::scope(|scope| {
            endpoints
                .iter()
                .map(|endpoint| {
                    scope.spawn(move || {
                        endpoint.address.map(|address| {
                            let started = Instant::now();
                            TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map(|_| {
                                started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32
                            })
                        })
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|worker| {
                    worker
                        .join()
                        .unwrap_or_else(|_| Some(Err(std::io::Error::other("채널 측정 작업 중단"))))
                })
                .collect::<Vec<_>>()
        });
        let measured_at = crate::now_ms();
        let mut failed = false;
        let mut runtime = self.runtime.write().unwrap();
        for (entry, result) in runtime.channels.iter_mut().zip(results) {
            let Some(result) = result else {
                entry.failed = false;
                continue;
            };
            entry.measured_at = Some(measured_at);
            match result {
                Ok(elapsed) => {
                    entry.failed = false;
                    entry.samples.push_back(elapsed);
                    while entry.samples.len() > SAMPLE_LIMIT {
                        entry.samples.pop_front();
                    }
                }
                Err(_) => {
                    entry.failed = true;
                    failed = true;
                }
            }
        }
        failed
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputDecision {
    None,
    Show(OverlayBounds),
    Hide,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InputEvent {
    Key {
        code: usize,
        down: bool,
        foreground: usize,
    },
    MouseDown,
}

static INPUT_EVENTS: Mutex<Option<SyncSender<InputEvent>>> = Mutex::new(None);
static INPUT_KEYS: Mutex<[bool; 256]> = Mutex::new([false; 256]);
static INPUT_OVERFLOW: AtomicBool = AtomicBool::new(false);
static INPUT_OVERLAY_WINDOW: Mutex<Option<(u64, isize)>> = Mutex::new(None);

pub fn set_input_overlay_window(window: Option<(u64, isize)>) {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock() {
        *registered = window;
    }
}

pub fn clear_input_overlay_window(window_id: u64) {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock()
        && registered.is_some_and(|(id, _)| id == window_id)
    {
        *registered = None;
    }
}

#[cfg(windows)]
fn hide_input_overlay() {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock()
        && let Some((_, hwnd)) = registered.take()
    {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindowAsync(
                hwnd as _,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
            );
        }
    }
}

pub struct InputMonitor {
    ignored: [bool; 256],
    events: Receiver<InputEvent>,
    blocked_until: Option<Instant>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl InputMonitor {
    pub fn new() -> Result<Self> {
        let (events, receiver) = sync_channel(256);
        let (ready, started) = sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let hook_stopping = stopping.clone();
        let thread = std::thread::spawn(move || input_hook_loop(events, hook_stopping, ready));
        let start = started.recv_timeout(Duration::from_secs(3));
        let start = match start {
            Ok(start) => start,
            Err(_) => {
                stopping.store(true, Ordering::Release);
                let _ = thread.join();
                return Err("입력 감시 시작 응답이 없습니다".into());
            }
        };
        if let Err(error) = start {
            stopping.store(true, Ordering::Release);
            let _ = thread.join();
            return Err(error);
        }
        Ok(Self {
            ignored: [false; 256],
            events: receiver,
            blocked_until: None,
            stopping,
            thread: Some(thread),
        })
    }

    pub fn poll(&mut self, visible: bool, env: &Environment) -> InputDecision {
        if take_input_overflow(&mut self.ignored) {
            self.blocked_until = Some(Instant::now() + Duration::from_millis(100));
            while self.events.try_recv().is_ok() {}
            return InputDecision::Hide;
        }
        if let Some(until) = self.blocked_until {
            let mut drained = false;
            while self.events.try_recv().is_ok() {
                drained = true;
            }
            if drained {
                self.blocked_until = Some(Instant::now() + Duration::from_millis(100));
                return if visible {
                    InputDecision::Hide
                } else {
                    InputDecision::None
                };
            }
            if Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
                return if visible {
                    InputDecision::Hide
                } else {
                    InputDecision::None
                };
            }
            self.blocked_until = None;
        }
        let event = match self.events.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return InputDecision::None,
            Err(RecvTimeoutError::Disconnected) => return InputDecision::Unavailable,
        };
        let bounds = match event {
            InputEvent::Key {
                code: 0x5b | 0x5c,
                down: true,
                foreground,
            } if !visible => game_bounds(env, foreground),
            _ => None,
        };
        decide_event(visible, event, &mut self.ignored, bounds)
    }
}

fn take_input_overflow(ignored: &mut [bool; 256]) -> bool {
    if !INPUT_OVERFLOW.swap(false, Ordering::AcqRel) {
        return false;
    }
    ignored.fill(false);
    true
}

impl Drop for InputMonitor {
    fn drop(&mut self) {
        set_input_overlay_window(None);
        self.stopping.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn decide_event(
    visible: bool,
    event: InputEvent,
    ignored: &mut [bool; 256],
    game_bounds: Option<OverlayBounds>,
) -> InputDecision {
    match event {
        InputEvent::Key {
            code, down: false, ..
        } => {
            if code < ignored.len() {
                ignored[code] = false;
            }
            InputDecision::None
        }
        InputEvent::Key {
            code, down: true, ..
        } if visible => {
            if code < ignored.len() && ignored[code] {
                InputDecision::None
            } else {
                InputDecision::Hide
            }
        }
        InputEvent::MouseDown if visible => InputDecision::Hide,
        InputEvent::Key {
            code: code @ (0x5b | 0x5c),
            down: true,
            ..
        } => game_bounds.map_or(InputDecision::None, |bounds| {
            ignored[code] = true;
            InputDecision::Show(bounds)
        }),
        _ => InputDecision::None,
    }
}

#[cfg(windows)]
unsafe extern "system" fn keyboard_hook(
    code: i32,
    parameter: windows_sys::Win32::Foundation::WPARAM,
    data: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };
    if code == HC_ACTION as i32 {
        let input = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
        let down = parameter == WM_KEYDOWN as usize || parameter == WM_SYSKEYDOWN as usize;
        let up = parameter == WM_KEYUP as usize || parameter == WM_SYSKEYUP as usize;
        if down || up {
            let key = input.vkCode as usize;
            let edge = INPUT_KEYS.lock().ok().is_some_and(|mut keys| {
                if key >= keys.len() || keys[key] == down {
                    return false;
                }
                keys[key] = down;
                true
            });
            if edge {
                if down {
                    hide_input_overlay();
                }
                send_input_event(InputEvent::Key {
                    code: key,
                    down,
                    foreground: if down {
                        (unsafe { GetForegroundWindow() }) as usize
                    } else {
                        0
                    },
                });
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, parameter, data) }
}

#[cfg(windows)]
unsafe extern "system" fn mouse_hook(
    code: i32,
    parameter: windows_sys::Win32::Foundation::WPARAM,
    data: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
    };
    if code == HC_ACTION as i32
        && matches!(
            parameter as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        hide_input_overlay();
        send_input_event(InputEvent::MouseDown);
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, parameter, data) }
}

#[cfg(windows)]
fn send_input_event(event: InputEvent) {
    if let Ok(sender) = INPUT_EVENTS.lock()
        && let Some(sender) = sender.as_ref()
    {
        if matches!(sender.try_send(event), Err(TrySendError::Full(_))) {
            INPUT_OVERFLOW.store(true, Ordering::Release);
        }
    }
}

#[cfg(windows)]
fn input_hook_loop(
    events: SyncSender<InputEvent>,
    stopping: Arc<AtomicBool>,
    ready: SyncSender<Result<()>>,
) {
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
            PeekMessageW, QS_ALLINPUT, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
            WH_KEYBOARD_LL, WH_MOUSE_LL,
        },
    };
    *INPUT_EVENTS.lock().unwrap() = Some(events);
    INPUT_KEYS.lock().unwrap().fill(false);
    INPUT_OVERFLOW.store(false, Ordering::Release);
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) };
    let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
    if keyboard.is_null() || mouse.is_null() {
        if !keyboard.is_null() {
            unsafe {
                UnhookWindowsHookEx(keyboard);
            }
        }
        if !mouse.is_null() {
            unsafe {
                UnhookWindowsHookEx(mouse);
            }
        }
        *INPUT_EVENTS.lock().unwrap() = None;
        let _ = ready.send(Err("키보드·마우스 입력 감시를 시작하지 못했습니다".into()));
        return;
    }
    let _ = ready.send(Ok(()));
    let mut message = unsafe { std::mem::zeroed::<MSG>() };
    while !stopping.load(Ordering::Acquire) {
        while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        unsafe {
            MsgWaitForMultipleObjectsEx(0, std::ptr::null(), 50, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }
    unsafe {
        UnhookWindowsHookEx(keyboard);
        UnhookWindowsHookEx(mouse);
    }
    *INPUT_EVENTS.lock().unwrap() = None;
    INPUT_KEYS.lock().unwrap().fill(false);
    INPUT_OVERFLOW.store(false, Ordering::Release);
}

#[cfg(windows)]
fn game_bounds(env: &Environment, window: usize) -> Option<OverlayBounds> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::HiDpi::GetDpiForWindow,
        UI::WindowsAndMessaging::{GetClientRect, GetWindowThreadProcessId},
    };
    let window = window as windows_sys::Win32::Foundation::HWND;
    if window.is_null() {
        return None;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let mut process = crate::process::list(false)
        .ok()?
        .into_iter()
        .find(|process| process.pid == pid)?;
    process.path = crate::process::path(pid).ok();
    let config = crate::workers::read(&env.root.join("config.json"));
    if !crate::process::matches_game(&process, &config) {
        return None;
    }
    let mut client = RECT::default();
    if unsafe { GetClientRect(window, &mut client) } == 0 {
        return None;
    }
    let mut origin = POINT {
        x: client.left,
        y: client.top,
    };
    if unsafe { ClientToScreen(window, &mut origin) } == 0 {
        return None;
    }
    let scale = f64::from(unsafe { GetDpiForWindow(window) }.max(96)) / 96.0;
    let width = 720.0;
    let height = 430.0;
    let physical_width = width * scale;
    let physical_height = height * scale;
    let client_width = f64::from(client.right - client.left);
    let client_height = f64::from(client.bottom - client.top);
    Some(OverlayBounds {
        x: f64::from(origin.x) + (client_width - physical_width) / 2.0,
        y: f64::from(origin.y) + (client_height - physical_height) / 2.0,
        width: physical_width,
        height: physical_height,
    })
}

#[cfg(not(windows))]
fn game_bounds(_env: &Environment, _window: usize) -> Option<OverlayBounds> {
    None
}

#[cfg(not(windows))]
fn input_hook_loop(
    _events: SyncSender<InputEvent>,
    _stopping: Arc<AtomicBool>,
    ready: SyncSender<Result<()>>,
) {
    let _ = ready.send(Err("Windows에서만 입력 감시를 사용할 수 있습니다".into()));
}

fn load_initial(cache: &Path, bundled: &Path) -> (Vec<Endpoint>, String, Option<String>) {
    let mut errors = Vec::new();
    for (path, source) in [(cache, "cache"), (bundled, "bundled")] {
        match std::fs::read_to_string(path) {
            Ok(text) => match parse_csv(&text) {
                Ok(channels) => return (channels, source.into(), None),
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    (
        Vec::new(),
        "unavailable".into(),
        (!errors.is_empty()).then(|| errors.join("; ")),
    )
}

fn measurements(endpoints: Vec<Endpoint>, previous: &[Measurement]) -> Vec<Measurement> {
    endpoints
        .into_iter()
        .map(|endpoint| {
            previous
                .iter()
                .find(|entry| entry.endpoint == endpoint)
                .cloned()
                .unwrap_or(Measurement {
                    endpoint,
                    samples: VecDeque::new(),
                    failed: false,
                    measured_at: None,
                })
        })
        .collect()
}

fn parse_csv(text: &str) -> Result<Vec<Endpoint>> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .map(|line| line.trim_start_matches('\u{feff}').trim())
        .ok_or("채널 정보 파일이 비어 있습니다")?;
    if header != "채널,IP:포트" {
        return Err("채널 정보 헤더가 올바르지 않습니다".into());
    }
    let mut channels = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (channel, endpoint) = line
            .split_once(',')
            .ok_or_else(|| format!("{}행의 채널 정보가 올바르지 않습니다", index + 2))?;
        let channel = channel
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|channel| (1..=100).contains(channel))
            .ok_or_else(|| format!("{}행의 채널 번호가 올바르지 않습니다", index + 2))?;
        let address = if endpoint.trim().is_empty() {
            None
        } else {
            let (ip, port) = endpoint
                .trim()
                .rsplit_once(':')
                .ok_or_else(|| format!("{}행의 IP와 포트가 올바르지 않습니다", index + 2))?;
            let ip = ip
                .parse::<IpAddr>()
                .map_err(|_| format!("{}행의 IP가 올바르지 않습니다", index + 2))?;
            if !public_unicast(ip) {
                return Err(format!(
                    "{}행의 IP는 공개 unicast 주소가 아닙니다",
                    index + 2
                ));
            }
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| format!("{}행의 포트가 올바르지 않습니다", index + 2))?;
            Some(SocketAddr::new(ip, port))
        };
        if channels
            .insert(channel, Endpoint { channel, address })
            .is_some()
        {
            return Err(format!("{channel}채널이 중복되었습니다"));
        }
    }
    let maximum = channels.keys().next_back().copied().unwrap_or(0);
    if maximum == 0 || channels.len() != usize::from(maximum) {
        return Err("채널 번호는 1부터 빠짐없이 포함되어야 합니다".into());
    }
    Ok(channels.into_values().collect())
}

fn public_unicast(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            a != 0
                && a != 10
                && a != 127
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 169 && b == 254)
                && !(a == 172 && (16..=31).contains(&b))
                && !(a == 192 && b == 168)
                && !(a == 192 && b == 0 && c == 2)
                && !(a == 198 && (b == 18 || b == 19 || b == 51 && c == 100))
                && !(a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                return public_unicast(IpAddr::V4(ip));
            }
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && ip.segments()[0..2] != [0x2001, 0x0db8]
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or("채널 정보 캐시 경로가 올바르지 않습니다")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".channel.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    static INPUT_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn csv는_빈_endpoint와_주소를_검증한다() {
        let channels = parse_csv(
            "\u{feff}채널,IP:포트\r\n1,211.218.233.210:11020\r\n2,\r\n3,211.218.233.211:11021\r\n",
        )
        .unwrap();
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].channel, 1);
        assert!(channels[1].address.is_none());
        assert_eq!(channels[2].address.unwrap().port(), 11021);
        assert!(parse_csv("채널,IP:포트\n1,127.0.0.1:80\n").is_err());
        assert!(parse_csv("채널,IP:포트\n1,::ffff:127.0.0.1:80\n").is_err());
    }

    #[test]
    fn csv는_중복과_누락_채널을_거부한다() {
        assert!(parse_csv("채널,IP:포트\n1,\n1,\n").is_err());
        assert!(parse_csv("채널,IP:포트\n1,\n3,\n").is_err());
    }

    #[test]
    fn endpoint가_같으면_기존_표본을_보존한다() {
        let endpoint = Endpoint {
            channel: 1,
            address: Some("211.218.233.210:11020".parse().unwrap()),
        };
        let previous = vec![Measurement {
            endpoint: endpoint.clone(),
            samples: VecDeque::from([10, 20]),
            failed: false,
            measured_at: Some(1),
        }];
        let retained = measurements(vec![endpoint.clone()], &previous);
        assert_eq!(retained[0].samples, VecDeque::from([10, 20]));
        let changed = measurements(
            vec![Endpoint {
                channel: 1,
                address: Some("211.218.233.210:11021".parse().unwrap()),
            }],
            &previous,
        );
        assert!(changed[0].samples.is_empty());
    }

    #[test]
    fn 표시_trigger는_release까지_무시하고_새_down은_숨긴다() {
        let mut ignored = [false; 256];
        let bounds = OverlayBounds {
            x: 0.0,
            y: 0.0,
            width: 720.0,
            height: 430.0,
        };
        assert_eq!(
            decide_event(
                false,
                InputEvent::Key {
                    code: 0x5b,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                Some(bounds)
            ),
            InputDecision::Show(bounds)
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x5b,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::None
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x5c,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::Hide
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x41,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::Hide
        );
    }

    #[test]
    fn 마우스_down은_표시된_창을_숨긴다() {
        assert_eq!(
            decide_event(true, InputEvent::MouseDown, &mut [false; 256], None),
            InputDecision::Hide
        );
    }

    #[test]
    fn 표시된_native_창_handle을_등록하고_해제한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        set_input_overlay_window(Some((7, 123)));
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), Some((7, 123)));
        clear_input_overlay_window(8);
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), Some((7, 123)));
        clear_input_overlay_window(7);
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), None);
        set_input_overlay_window(None);
    }

    #[test]
    #[cfg(windows)]
    fn 저수준_input_hook을_설치하고_정리한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let monitor = InputMonitor::new().unwrap();
        drop(monitor);
        assert!(INPUT_EVENTS.lock().unwrap().is_none());
    }

    #[test]
    fn input_queue_포화는_stale_event를_비우고_hide한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let (sender, events) = sync_channel(4);
        sender
            .send(InputEvent::Key {
                code: 0x5b,
                down: true,
                foreground: 1,
            })
            .unwrap();
        sender.send(InputEvent::MouseDown).unwrap();
        let mut monitor = InputMonitor {
            ignored: [true; 256],
            events,
            blocked_until: None,
            stopping: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        INPUT_OVERFLOW.store(true, Ordering::Release);
        let path = PathBuf::new();
        let env = Environment {
            root: path.clone(),
            user: path.clone(),
            documents: path.clone(),
            videos: path.clone(),
            exe: path,
            packaged: false,
            portable: false,
        };
        assert_eq!(monitor.poll(true, &env), InputDecision::Hide);
        assert!(monitor.ignored.iter().all(|value| !value));
        assert!(matches!(
            monitor.events.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        sender.send(InputEvent::MouseDown).unwrap();
        assert_eq!(monitor.poll(false, &env), InputDecision::None);
        assert!(matches!(
            monitor.events.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn input_channel_단절은_재시작_신호를_반환한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let (sender, events) = sync_channel(1);
        drop(sender);
        let mut monitor = InputMonitor {
            ignored: [false; 256],
            events,
            blocked_until: None,
            stopping: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        let path = PathBuf::new();
        let env = Environment {
            root: path.clone(),
            user: path.clone(),
            documents: path.clone(),
            videos: path.clone(),
            exe: path,
            packaged: false,
            portable: false,
        };
        assert_eq!(monitor.poll(false, &env), InputDecision::Unavailable);
    }
}
