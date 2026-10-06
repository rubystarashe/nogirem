use crate::{
    Result,
    overlay_geometry::OverlayBounds,
    process::{self, Process},
    storage,
    workers::{Environment, read},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettings {
    pub debuff_notifications: bool,
    pub tuarim_notifications: bool,
    pub collapse_notifications: bool,
    pub buff_notifications: bool,
    pub skill_notifications: bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            debuff_notifications: false,
            tuarim_notifications: false,
            collapse_notifications: false,
            buff_notifications: false,
            skill_notifications: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Detection {
    window: isize,
    pid: u32,
    process_started_at: i64,
    rect: PixelRect,
}

pub struct GameOverlay {
    path: PathBuf,
    settings: OverlaySettings,
    detection: Option<Detection>,
    error: Option<String>,
}

impl GameOverlay {
    pub fn new(env: &Environment) -> Self {
        let path = env.user.join("overlay/settings.json");
        let settings = serde_json::from_value(read(&path)).unwrap_or_default();
        Self {
            path,
            settings,
            detection: None,
            error: None,
        }
    }

    pub fn status(&self) -> Value {
        let mut value = serde_json::to_value(&self.settings).unwrap_or_else(|_| json!({}));
        value["debuffRegionDetected"] = json!(self.detection.is_some());
        value["detectionError"] = self
            .error
            .as_ref()
            .map_or(Value::Null, |error| json!(error));
        if let Some(detection) = self.detection {
            value["debuffRegion"] = json!({
                "x": detection.rect.x,
                "y": detection.rect.y,
                "width": detection.rect.width,
                "height": detection.rect.height
            });
        }
        value
    }

    pub fn set(&mut self, value: &Value) -> Result<Value> {
        let object = value
            .as_object()
            .ok_or("오버레이 설정 값이 올바르지 않습니다")?;
        let mut next = self.settings.clone();
        for (key, target) in [
            ("debuffNotifications", &mut next.debuff_notifications),
            ("tuarimNotifications", &mut next.tuarim_notifications),
            ("collapseNotifications", &mut next.collapse_notifications),
            ("buffNotifications", &mut next.buff_notifications),
            ("skillNotifications", &mut next.skill_notifications),
        ] {
            if let Some(value) = object.get(key) {
                *target = value
                    .as_bool()
                    .ok_or_else(|| format!("{key} 설정 값이 올바르지 않습니다"))?;
            }
        }
        if object.get("debuffNotifications") == Some(&Value::Bool(false)) {
            self.detection = None;
            self.error = None;
        }
        storage::write_json(
            &self.path,
            &serde_json::to_value(&next).map_err(|e| e.to_string())?,
        )?;
        self.settings = next;
        Ok(self.status())
    }

    pub fn detect_debuff_region(&mut self, env: &Environment) -> Result<Value> {
        self.detection = None;
        self.error = None;
        let result = capture_game_client(env).and_then(|capture| {
            let rect = detect_boss_health_bar(&capture.frame)
                .ok_or_else(|| "보스 체력바 영역을 찾지 못했습니다".to_owned())?;
            self.detection = Some(Detection {
                window: capture.window,
                pid: capture.pid,
                process_started_at: capture.process_started_at,
                rect,
            });
            Ok(self.status())
        });
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub fn preview_bounds(&mut self, env: &Environment) -> Option<OverlayBounds> {
        let detection = self.detection?;
        let bounds = current_screen_bounds(env, detection)?;
        Some(OverlayBounds {
            x: bounds.x - 4.0,
            y: bounds.y - 24.0,
            width: bounds.width + 8.0,
            height: bounds.height + 28.0,
        })
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

#[derive(Clone, Copy)]
struct ScreenRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

struct Capture {
    window: isize,
    pid: u32,
    process_started_at: i64,
    frame: Frame,
}

fn normalize_process(env: &Environment, window: isize) -> Option<(u32, i64)> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window as _, &mut pid);
        }
        if pid == 0 {
            return None;
        }
        let path = process::path(pid).ok()?;
        let info = Process {
            pid,
            name: Path::new(&path).file_name()?.to_string_lossy().into_owned(),
            path: Some(path),
            start_time: None,
            session_id: None,
        };
        let config = read(&env.root.join("config.json"));
        if !process::matches_game(&info, &config) {
            return None;
        }
        Some((pid, process::start_ms(pid).ok()?))
    }
    #[cfg(not(windows))]
    {
        let _ = (env, window);
        None
    }
}

#[cfg(windows)]
fn find_game_window(env: &Environment) -> Result<(isize, u32, i64)> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{EnumWindows, GetForegroundWindow, IsIconic, IsWindowVisible},
    };

    unsafe extern "system" fn collect(window: HWND, parameter: LPARAM) -> i32 {
        let windows = unsafe { &mut *(parameter as *mut Vec<isize>) };
        windows.push(window as isize);
        1
    }

    let foreground = unsafe { GetForegroundWindow() } as isize;
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(Some(collect), &mut windows as *mut Vec<isize> as LPARAM);
    }
    windows.sort_by_key(|window| (*window != foreground) as u8);
    let mut candidates = Vec::new();
    for window in windows {
        if unsafe { IsWindowVisible(window as _) } == 0 || unsafe { IsIconic(window as _) } != 0 {
            continue;
        }
        if let Some((pid, started_at)) = normalize_process(env, window) {
            candidates.push((window, pid, started_at));
        }
    }
    if candidates.is_empty() {
        return Err("실행 중이며 최소화되지 않은 마비노기 창을 찾지 못했습니다".into());
    }
    if candidates.len() == 1 || candidates[0].0 == foreground {
        return Ok(candidates[0]);
    }
    let expected = env.game().replace('/', "\\");
    let exact: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|(_, pid, _)| {
            !expected.is_empty()
                && process::path(*pid)
                    .is_ok_and(|path| path.replace('/', "\\").eq_ignore_ascii_case(&expected))
        })
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0]);
    }
    Err("마비노기 창이 여러 개라 자동인식 대상을 결정하지 못했습니다".into())
}

#[cfg(not(windows))]
fn find_game_window(_env: &Environment) -> Result<(isize, u32, i64)> {
    Err("Windows에서만 게임 화면을 자동인식할 수 있습니다".into())
}

#[cfg(windows)]
fn activate_window(window: isize) -> bool {
    use windows_sys::Win32::{
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        },
    };
    let window = window as windows_sys::Win32::Foundation::HWND;
    if window.is_null() {
        return false;
    }
    if unsafe { GetForegroundWindow() } == window {
        return true;
    }
    let current_thread = unsafe { GetCurrentThreadId() };
    let foreground = unsafe { GetForegroundWindow() };
    let foreground_thread = if foreground.is_null() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) }
    };
    let target_thread = unsafe { GetWindowThreadProcessId(window, std::ptr::null_mut()) };
    let attach_foreground = foreground_thread != 0 && foreground_thread != current_thread;
    let attach_target =
        target_thread != 0 && target_thread != current_thread && target_thread != foreground_thread;
    if attach_foreground {
        unsafe {
            AttachThreadInput(current_thread, foreground_thread, 1);
        }
    }
    if attach_target {
        unsafe {
            AttachThreadInput(current_thread, target_thread, 1);
        }
    }
    let activated =
        unsafe { BringWindowToTop(window) } != 0 && unsafe { SetForegroundWindow(window) } != 0;
    if attach_target {
        unsafe {
            AttachThreadInput(current_thread, target_thread, 0);
        }
    }
    if attach_foreground {
        unsafe {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
    activated && unsafe { GetForegroundWindow() } == window
}

#[cfg(windows)]
fn capture_game_client(env: &Environment) -> Result<Capture> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, ClientToScreen,
            CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
            GetDC, GetDIBits, ReleaseDC, SRCCOPY, SelectObject,
        },
        UI::WindowsAndMessaging::{GetClientRect, GetForegroundWindow},
    };

    let (window, pid, process_started_at) = find_game_window(env)?;
    let previous = unsafe { GetForegroundWindow() };
    if !activate_window(window) {
        return Err("마비노기 창을 캡처 대상으로 전환하지 못했습니다".into());
    }
    std::thread::sleep(std::time::Duration::from_millis(180));
    if unsafe { GetForegroundWindow() } as isize != window {
        if !previous.is_null() {
            activate_window(previous as isize);
        }
        return Err("마비노기 창이 전경으로 전환되지 않아 캡처를 중단했습니다".into());
    }

    let result = (|| {
        let mut client = RECT::default();
        if unsafe { GetClientRect(window as _, &mut client) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let width = client.right - client.left;
        let height = client.bottom - client.top;
        if width <= 0 || height <= 0 {
            return Err("마비노기 화면 크기를 확인하지 못했습니다".into());
        }
        let mut origin = POINT { x: 0, y: 0 };
        if unsafe { ClientToScreen(window as _, &mut origin) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let source_window = std::ptr::null_mut();
        let source = unsafe { GetDC(source_window) };
        if source.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let memory = unsafe { CreateCompatibleDC(source) };
        if memory.is_null() {
            unsafe {
                ReleaseDC(source_window, source);
            }
            return Err(std::io::Error::last_os_error().to_string());
        }
        let bitmap = unsafe { CreateCompatibleBitmap(source, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory);
                ReleaseDC(source_window, source);
            }
            return Err(std::io::Error::last_os_error().to_string());
        }
        let previous_bitmap = unsafe { SelectObject(memory, bitmap as _) };
        let copied = unsafe {
            BitBlt(
                memory,
                origin.x,
                origin.y,
                width,
                height,
                source,
                0,
                0,
                SRCCOPY | CAPTUREBLT,
            )
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            SelectObject(memory, previous_bitmap);
        }
        let rows = if copied != 0 {
            unsafe {
                GetDIBits(
                    memory,
                    bitmap,
                    0,
                    height as u32,
                    pixels.as_mut_ptr() as _,
                    &mut info,
                    DIB_RGB_COLORS,
                )
            }
        } else {
            0
        };
        unsafe {
            DeleteObject(bitmap as _);
            DeleteDC(memory);
            ReleaseDC(source_window, source);
        }
        if rows != height {
            return Err("마비노기 화면 픽셀을 읽지 못했습니다".into());
        }
        if normalize_process(env, window) != Some((pid, process_started_at)) {
            return Err("캡처 중 마비노기 프로세스가 변경되었습니다".into());
        }
        Ok(Capture {
            window,
            pid,
            process_started_at,
            frame: Frame {
                width: width as u32,
                height: height as u32,
                bgra: pixels,
            },
        })
    })();

    if !previous.is_null() {
        activate_window(previous as isize);
    }
    result
}

#[cfg(not(windows))]
fn capture_game_client(_env: &Environment) -> Result<Capture> {
    Err("Windows에서만 게임 화면을 자동인식할 수 있습니다".into())
}

#[cfg(windows)]
fn current_screen_bounds(_env: &Environment, detection: Detection) -> Option<ScreenRect> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            GetClientRect, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
            IsWindowVisible,
        },
    };
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(detection.window as _, &mut pid);
    }
    if unsafe { IsWindow(detection.window as _) } == 0
        || unsafe { IsWindowVisible(detection.window as _) } == 0
        || unsafe { IsIconic(detection.window as _) } != 0
        || unsafe { GetForegroundWindow() } as isize != detection.window
        || pid != detection.pid
        || process::start_ms(pid).ok() != Some(detection.process_started_at)
    {
        return None;
    }
    let mut client = RECT::default();
    if unsafe { GetClientRect(detection.window as _, &mut client) } == 0 {
        return None;
    }
    let mut origin = POINT { x: 0, y: 0 };
    if unsafe { ClientToScreen(detection.window as _, &mut origin) } == 0 {
        return None;
    }
    if detection.rect.x + detection.rect.width > (client.right - client.left) as u32
        || detection.rect.y + detection.rect.height > (client.bottom - client.top) as u32
    {
        return None;
    }
    Some(ScreenRect {
        x: f64::from(origin.x) + f64::from(detection.rect.x),
        y: f64::from(origin.y) + f64::from(detection.rect.y),
        width: f64::from(detection.rect.width),
        height: f64::from(detection.rect.height),
    })
}

#[cfg(not(windows))]
fn current_screen_bounds(_env: &Environment, _detection: Detection) -> Option<ScreenRect> {
    None
}

#[derive(Clone, Copy)]
struct Run {
    start: u32,
    end: u32,
}

impl Run {
    fn width(self) -> u32 {
        self.end - self.start + 1
    }
}

fn pixel(frame: &Frame, x: u32, y: u32) -> Option<[u8; 3]> {
    let index = (y as usize * frame.width as usize + x as usize) * 4;
    let data = frame.bgra.get(index..index + 3)?;
    Some([data[2], data[1], data[0]])
}

fn gray(pixel: [u8; 3], minimum: u8, maximum: u8, channel_spread: u8) -> bool {
    pixel
        .iter()
        .all(|value| (minimum..=maximum).contains(value))
        && pixel.iter().max().unwrap() - pixel.iter().min().unwrap() <= channel_spread
}

fn runs(frame: &Frame, y: u32, minimum: u8, maximum: u8, minimum_width: u32) -> Vec<Run> {
    let mut result = Vec::new();
    let mut start = None;
    for x in 0..frame.width {
        if pixel(frame, x, y).is_some_and(|value| gray(value, minimum, maximum, 2)) {
            start.get_or_insert(x);
        } else if let Some(run_start) = start.take() {
            if x - run_start >= minimum_width {
                result.push(Run {
                    start: run_start,
                    end: x - 1,
                });
            }
        }
    }
    if let Some(run_start) = start {
        if frame.width - run_start >= minimum_width {
            result.push(Run {
                start: run_start,
                end: frame.width - 1,
            });
        }
    }
    result
}

fn close(a: u32, b: u32, tolerance: u32) -> bool {
    a.abs_diff(b) <= tolerance
}

fn vertical_black_ratio(frame: &Frame, x: u32, top: u32, bottom: u32) -> f64 {
    let total = bottom - top + 1;
    let black = (top..=bottom)
        .filter(|y| pixel(frame, x, *y).is_some_and(|value| gray(value, 0, 2, 2)))
        .count() as u32;
    f64::from(black) / f64::from(total)
}

fn vertical_edge(frame: &Frame, expected: u32, top: u32, bottom: u32) -> Option<(u32, f64)> {
    (expected.saturating_sub(2)
        ..=expected
            .saturating_add(2)
            .min(frame.width.saturating_sub(1)))
        .map(|x| (x, vertical_black_ratio(frame, x, top, bottom)))
        .filter(|(_, ratio)| *ratio >= 0.95)
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

pub fn detect_boss_health_bar(frame: &Frame) -> Option<PixelRect> {
    if frame.width < 150
        || frame.height < 23
        || frame.bgra.len() < frame.width as usize * frame.height as usize * 4
    {
        return None;
    }
    let mut candidates = Vec::new();
    for bottom in 0..frame.height.saturating_sub(1) {
        for lower in runs(frame, bottom, 53, 57, 150) {
            for darkest in runs(frame, bottom + 1, 15, 19, 150) {
                if !close(lower.start, darkest.start, 2)
                    || !close(lower.end, darkest.end, 2)
                    || lower.width().abs_diff(darkest.width()) > 2
                {
                    continue;
                }
                let minimum_top = bottom.saturating_sub(120);
                let maximum_top = bottom.saturating_sub(20);
                for top in minimum_top..=maximum_top {
                    for upper in runs(frame, top, 7, 12, 150) {
                        if !close(lower.start, upper.start, 2)
                            || !close(lower.end, upper.end, 2)
                            || lower.width().abs_diff(upper.width()) > 2
                        {
                            continue;
                        }
                        let Some(left) = vertical_edge(frame, lower.start, top, bottom + 1) else {
                            continue;
                        };
                        let Some(right) = vertical_edge(frame, lower.end, top, bottom + 1) else {
                            continue;
                        };
                        if right.0 <= left.0 {
                            continue;
                        }
                        let width = right.0 - left.0 + 1;
                        let height = bottom + 2 - top;
                        if width < height.saturating_mul(5) {
                            continue;
                        }
                        let endpoint_error =
                            lower.start.abs_diff(upper.start) + lower.end.abs_diff(upper.end);
                        let score = left.1 + right.1 - f64::from(endpoint_error) * 0.02
                            + f64::from(width) / f64::from(frame.width);
                        candidates.push((
                            score,
                            PixelRect {
                                x: left.0,
                                y: top,
                                width,
                                height,
                            },
                        ));
                    }
                }
            }
        }
    }
    candidates
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, rect)| rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(width: u32, height: u32, rect: PixelRect) -> Frame {
        let mut frame = Frame {
            width,
            height,
            bgra: vec![200; width as usize * height as usize * 4],
        };
        for pixel in frame.bgra.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let mut set = |x: u32, y: u32, value: u8| {
            let index = (y as usize * width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(value);
        };
        let right = rect.x + rect.width - 1;
        let bottom = rect.y + rect.height - 1;
        for x in rect.x..=right {
            set(x, rect.y, 10);
            set(x, bottom - 1, 55);
            set(x, bottom, 17);
        }
        for y in rect.y..=bottom {
            set(rect.x, y, 0);
            set(right, y, 0);
        }
        frame
    }

    #[test]
    fn 가변_높이의_보스_체력바_외곽을_찾는다() {
        for height in [22, 40, 61, 100] {
            let expected = PixelRect {
                x: 80,
                y: 80,
                width: 402.max(height * 6),
                height,
            };
            assert_eq!(
                detect_boss_health_bar(&fixture(800, 240, expected)),
                Some(expected)
            );
        }
    }

    #[test]
    fn 상하단선_길이가_두_픽셀_차이나도_찾는다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut frame = fixture(640, 180, expected);
        for x in expected.x..expected.x + 2 {
            let index = (expected.y as usize * frame.width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(0);
        }
        assert_eq!(detect_boss_health_bar(&frame), Some(expected));
    }

    #[test]
    fn 제공된_체력바의_비대칭_rgb와_상단_경계_gap을_허용한다() {
        let expected = PixelRect {
            x: 0,
            y: 0,
            width: 523,
            height: 41,
        };
        let mut frame = Frame {
            width: 523,
            height: 41,
            bgra: vec![120; 523 * 41 * 4],
        };
        for pixel in frame.bgra.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let mut set = |x: u32, y: u32, red: u8, green: u8, blue: u8| {
            let index = (y as usize * 523 + x as usize) * 4;
            frame.bgra[index..index + 3].copy_from_slice(&[blue, green, red]);
        };
        for x in 1..522 {
            set(x, 0, 9, 10, 10);
            set(x, 39, 55, 55, 55);
            set(x, 40, 17, 17, 17);
        }
        for y in 2..=40 {
            set(0, y, 0, 0, 0);
            set(522, y, 0, 0, 0);
        }
        assert_eq!(detect_boss_health_bar(&frame), Some(expected));
    }

    #[test]
    fn 이중선이나_검은_세로선이_없으면_거부한다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut no_double = fixture(640, 180, expected);
        let bottom = expected.y + expected.height - 1;
        for x in expected.x..expected.x + expected.width {
            let index = (bottom as usize * no_double.width as usize + x as usize) * 4;
            no_double.bgra[index..index + 3].fill(80);
        }
        assert_eq!(detect_boss_health_bar(&no_double), None);

        let mut no_sides = fixture(640, 180, expected);
        for y in expected.y..expected.y + expected.height {
            for x in [expected.x, expected.x + expected.width - 1] {
                let index = (y as usize * no_sides.width as usize + x as usize) * 4;
                no_sides.bgra[index..index + 3].fill(80);
            }
        }
        assert_eq!(detect_boss_health_bar(&no_sides), None);
    }

    #[test]
    fn 설정은_알려진_boolean만_반영하고_디버프를_끄면_검출을_지운다() {
        let root = std::env::temp_dir().join(format!("nogirem-overlay-{}", uuid::Uuid::new_v4()));
        let env = Environment {
            root: root.clone(),
            user: root.join("user"),
            documents: root.join("documents"),
            videos: root.join("videos"),
            exe: root.join("nogirem.exe"),
            packaged: false,
            portable: false,
        };
        let mut overlay = GameOverlay::new(&env);
        let status = overlay
            .set(&json!({"debuffNotifications":true,"skillNotifications":true}))
            .unwrap();
        assert_eq!(status["debuffNotifications"], true);
        assert_eq!(status["skillNotifications"], true);
        assert_eq!(status["tuarimNotifications"], false);
        overlay.detection = Some(Detection {
            window: 1,
            pid: 2,
            process_started_at: 3,
            rect: PixelRect {
                x: 4,
                y: 5,
                width: 300,
                height: 40,
            },
        });
        let status = overlay.set(&json!({"debuffNotifications":false})).unwrap();
        assert_eq!(status["debuffRegionDetected"], false);
        overlay.detection = Some(Detection {
            window: 1,
            pid: 2,
            process_started_at: 3,
            rect: PixelRect {
                x: 4,
                y: 5,
                width: 300,
                height: 40,
            },
        });
        let status = overlay.set(&json!({"skillNotifications":false})).unwrap();
        assert_eq!(status["debuffRegionDetected"], true);
        std::fs::remove_dir_all(root).unwrap();
    }
}
