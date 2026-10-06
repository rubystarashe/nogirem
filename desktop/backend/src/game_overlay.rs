use crate::{
    Result,
    overlay_geometry::OverlayBounds,
    process::{self, Process},
    storage,
    workers::{Environment, read},
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct OverlaySettings {
    pub debuff_notifications: bool,
    pub tuarim_notifications: bool,
    pub collapse_notifications: bool,
    pub buff_notifications: bool,
    pub skill_notifications: bool,
    pub debuff_alert_offset_x: Option<i32>,
    pub debuff_alert_offset_y: Option<i32>,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            debuff_notifications: false,
            tuarim_notifications: false,
            collapse_notifications: false,
            buff_notifications: false,
            skill_notifications: false,
            debuff_alert_offset_x: None,
            debuff_alert_offset_y: None,
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

#[derive(Clone)]
struct DebuffTemplate {
    id: u32,
    kind: u8,
    name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    anchor_x: u32,
    anchor_y: u32,
    anchor_rgb: [u8; 3],
    icon: String,
}

pub struct GameOverlay {
    path: PathBuf,
    settings: OverlaySettings,
    detection: Option<Detection>,
    error: Option<String>,
    templates: Option<Vec<DebuffTemplate>>,
    template_error: Option<String>,
    missing_debuffs: Vec<usize>,
    configuring_alert: bool,
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
            templates: None,
            template_error: None,
            missing_debuffs: Vec::new(),
            configuring_alert: false,
        }
    }

    pub fn status(&self) -> Value {
        let mut value = serde_json::to_value(&self.settings).unwrap_or_else(|_| json!({}));
        value["debuffRegionDetected"] = json!(self.detection.is_some());
        value["detectionError"] = self
            .error
            .as_ref()
            .map_or(Value::Null, |error| json!(error));
        value["debuffAlertConfiguring"] = json!(self.configuring_alert);
        value["debuffTemplateError"] = self
            .template_error
            .as_ref()
            .map_or(Value::Null, |error| json!(error));
        value["missingDebuffs"] = self
            .templates
            .as_ref()
            .map(|templates| {
                self.missing_debuffs
                    .iter()
                    .filter_map(|index| templates.get(*index))
                    .map(|template| {
                        json!({
                            "id": template.id,
                            "type": template.kind,
                            "name": template.name,
                            "icon": template.icon
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .map_or_else(|| json!([]), Value::from);
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
            self.missing_debuffs.clear();
            self.configuring_alert = false;
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
            let rect = detect_boss_health_bar(&capture.frame).ok_or_else(|| {
                format!(
                    "보스 체력바 영역을 찾지 못했습니다 ({})",
                    detector_diagnostics(&capture.frame)
                )
            })?;
            self.detection = Some(Detection {
                window: capture.window,
                pid: capture.pid,
                process_started_at: capture.process_started_at,
                rect,
            });
            self.missing_debuffs.clear();
            Ok(self.status())
        });
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub fn preview_bounds(&mut self, env: &Environment) -> Option<OverlayBounds> {
        if !self.configuring_alert {
            return None;
        }
        let detection = self.detection?;
        let bounds = screen_bounds(env, detection, false)?;
        Some(OverlayBounds {
            x: bounds.x - 4.0,
            y: bounds.y - bounds.height - 24.0,
            width: bounds.width + 8.0,
            height: bounds.height + 28.0,
        })
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn configure_alert(&mut self, enabled: bool) -> Result<Value> {
        self.configuring_alert = enabled;
        Ok(self.status())
    }

    pub fn save_alert_position(
        &mut self,
        env: &Environment,
        screen_x: i32,
        screen_y: i32,
    ) -> Result<()> {
        let anchor = game_client_bounds(env, self.detection, false)
            .ok_or("마비노기 창 위치를 확인하지 못했습니다")?;
        self.settings.debuff_alert_offset_x = Some((f64::from(screen_x) - anchor.x).round() as i32);
        self.settings.debuff_alert_offset_y = Some((f64::from(screen_y) - anchor.y).round() as i32);
        storage::write_json(
            &self.path,
            &serde_json::to_value(&self.settings).map_err(|error| error.to_string())?,
        )
    }

    pub fn alert_bounds(&mut self, env: &Environment) -> Option<OverlayBounds> {
        let anchor = game_client_bounds(env, self.detection, !self.configuring_alert)?;
        Some(OverlayBounds {
            x: anchor.x + f64::from(self.settings.debuff_alert_offset_x.unwrap_or(32)),
            y: anchor.y + f64::from(self.settings.debuff_alert_offset_y.unwrap_or(32)),
            width: 520.0,
            height: 440.0,
        })
    }

    pub fn should_show_alert(&self) -> bool {
        self.configuring_alert
            || (self.settings.debuff_notifications && !self.missing_debuffs.is_empty())
    }

    pub fn alert_configuring(&self) -> bool {
        self.configuring_alert
    }

    pub fn scan_debuffs(&mut self, env: &Environment) -> Result<bool> {
        if !self.settings.debuff_notifications || self.configuring_alert {
            return Ok(false);
        }
        let detection = self
            .detection
            .ok_or("보스 체력바 영역이 설정되지 않았습니다")?;
        let source = debuff_source_rect(detection.rect)
            .ok_or("보스 체력바 위 디버프 영역이 게임 화면 밖입니다")?;
        let frame = capture_detected_region(env, detection, source)?;
        if self.templates.is_none() {
            match load_debuff_templates(env) {
                Ok(templates) => {
                    self.templates = Some(templates);
                    self.template_error = None;
                }
                Err(error) => {
                    self.template_error = Some(error.clone());
                    return Err(error);
                }
            }
        }
        let templates = self.templates.as_ref().unwrap();
        let missing = templates
            .iter()
            .enumerate()
            .filter_map(|(index, template)| (!template_present(&frame, template)).then_some(index))
            .collect::<Vec<_>>();
        let changed = missing != self.missing_debuffs;
        self.missing_debuffs = missing;
        Ok(changed)
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

fn load_debuff_templates(env: &Environment) -> Result<Vec<DebuffTemplate>> {
    load_debuff_templates_from_root(&env.root)
}

fn load_debuff_templates_from_root(root: &Path) -> Result<Vec<DebuffTemplate>> {
    let csv = std::fs::read_to_string(root.join("debuff.csv"))
        .map_err(|error| format!("디버프 목록을 읽지 못했습니다: {error}"))?;
    let mut templates: Vec<DebuffTemplate> = Vec::new();
    for (line_index, line) in csv.lines().enumerate().skip(1) {
        let mut fields = line.split(',').map(str::trim);
        let id = fields
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| format!("debuff.csv {}행 ID가 올바르지 않습니다", line_index + 1))?;
        let kind = fields
            .next()
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|value| (1..=6).contains(value))
            .ok_or_else(|| format!("debuff.csv {}행 type이 올바르지 않습니다", line_index + 1))?;
        let name = fields
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("debuff.csv {}행 name이 비어 있습니다", line_index + 1))?;
        if fields.next().is_some() || templates.iter().any(|template| template.id == id) {
            return Err(format!(
                "debuff.csv {}행이 중복되었거나 형식이 올바르지 않습니다",
                line_index + 1
            ));
        }
        let bytes = std::fs::read(root.join(format!("data/conditions/{id}.png")))
            .map_err(|error| format!("디버프 {id} 아이콘을 읽지 못했습니다: {error}"))?;
        let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|error| format!("디버프 {id} 아이콘이 올바른 PNG가 아닙니다: {error}"))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        if width != 16 || height != 16 {
            return Err(format!("디버프 {id} 아이콘 크기는 16×16이어야 합니다"));
        }
        let anchor = image
            .pixels()
            .enumerate()
            .filter(|(_, pixel)| pixel[3] != 0)
            .max_by_key(|(_, pixel)| {
                let high = pixel[0].max(pixel[1]).max(pixel[2]);
                let low = pixel[0].min(pixel[1]).min(pixel[2]);
                u16::from(high - low) * 2 + u16::from(high)
            })
            .ok_or_else(|| format!("디버프 {id} 아이콘에 불투명 픽셀이 없습니다"))?;
        let anchor_index =
            u32::try_from(anchor.0).map_err(|_| "디버프 아이콘 좌표가 너무 큽니다")?;
        let anchor_rgb = [anchor.1[0], anchor.1[1], anchor.1[2]];
        templates.push(DebuffTemplate {
            id,
            kind,
            name: name.to_owned(),
            width,
            height,
            rgba: image.into_raw(),
            anchor_x: anchor_index % width,
            anchor_y: anchor_index / width,
            anchor_rgb,
            icon: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
        });
    }
    if templates.is_empty() {
        return Err("debuff.csv에 판독할 디버프가 없습니다".into());
    }
    Ok(templates)
}

fn debuff_source_rect(health: PixelRect) -> Option<PixelRect> {
    Some(PixelRect {
        x: health.x,
        y: health.y.checked_sub(health.height)?,
        width: health.width,
        height: health.height,
    })
}

fn template_matches(frame: &Frame, template: &DebuffTemplate, x: u32, y: u32, scale: u32) -> bool {
    if scale != 1
        || !pixel(frame, x + template.anchor_x, y + template.anchor_y)
            .is_some_and(|sample| rgb_matches(sample, template.anchor_rgb))
    {
        return false;
    }
    let mut compared = 0u32;
    for ty in 0..template.height {
        for tx in 0..template.width {
            let template_index = (ty as usize * template.width as usize + tx as usize) * 4;
            let alpha = template.rgba[template_index + 3];
            let red = template.rgba[template_index];
            let green = template.rgba[template_index + 1];
            let blue = template.rgba[template_index + 2];
            if alpha == 0 {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let Some(sample) = pixel(frame, x + tx * scale + sx, y + ty * scale + sy)
                    else {
                        return false;
                    };
                    if !rgb_matches(sample, [red, green, blue]) {
                        return false;
                    }
                    compared += 1;
                }
            }
        }
    }
    compared > 0
}

fn rgb_matches(sample: [u8; 3], template: [u8; 3]) -> bool {
    sample
        .into_iter()
        .zip(template)
        .all(|(sample, template)| sample.abs_diff(template) <= 2)
}

fn template_present(frame: &Frame, template: &DebuffTemplate) -> bool {
    if template.width > frame.width || template.height > frame.height {
        return false;
    }
    for y in 0..=frame.height - template.height {
        for x in 0..=frame.width - template.width {
            if template_matches(frame, template, x, y, 1) {
                return true;
            }
        }
    }
    false
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

#[cfg(windows)]
fn capture_detected_region(
    env: &Environment,
    detection: Detection,
    rect: PixelRect,
) -> Result<Frame> {
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    let bounds = screen_bounds(env, Detection { rect, ..detection }, true)
        .ok_or("마비노기 창이 전경이 아니거나 디버프 영역이 유효하지 않습니다")?;
    let width = i32::try_from(rect.width).map_err(|_| "디버프 영역 너비가 너무 큽니다")?;
    let height = i32::try_from(rect.height).map_err(|_| "디버프 영역 높이가 너무 큽니다")?;
    let source_window = std::ptr::null_mut();
    let source = unsafe { GetDC(source_window) };
    if source.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let memory = unsafe { CreateCompatibleDC(source) };
    let bitmap = if memory.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { CreateCompatibleBitmap(source, width, height) }
    };
    if memory.is_null() || bitmap.is_null() {
        if !memory.is_null() {
            unsafe {
                DeleteDC(memory);
            }
        }
        unsafe {
            ReleaseDC(source_window, source);
        }
        return Err(std::io::Error::last_os_error().to_string());
    }
    let previous = unsafe { SelectObject(memory, bitmap as _) };
    let copied = unsafe {
        BitBlt(
            memory,
            0,
            0,
            width,
            height,
            source,
            bounds.x.round() as i32,
            bounds.y.round() as i32,
            SRCCOPY | CAPTUREBLT,
        )
    };
    unsafe {
        SelectObject(memory, previous);
    }
    let mut pixels = vec![0u8; rect.width as usize * rect.height as usize * 4];
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
    let rows = if copied == 0 {
        0
    } else {
        unsafe {
            GetDIBits(
                memory,
                bitmap,
                0,
                rect.height,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            )
        }
    };
    unsafe {
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(source_window, source);
    }
    if copied == 0 || rows != rect.height as i32 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(Frame {
        width: rect.width,
        height: rect.height,
        bgra: pixels,
    })
}

#[cfg(not(windows))]
fn capture_detected_region(
    _env: &Environment,
    _detection: Detection,
    _rect: PixelRect,
) -> Result<Frame> {
    Err("Windows에서만 디버프 영역을 캡처할 수 있습니다".into())
}

#[cfg(not(windows))]
fn capture_game_client(_env: &Environment) -> Result<Capture> {
    Err("Windows에서만 게임 화면을 자동인식할 수 있습니다".into())
}

#[cfg(windows)]
fn game_client_bounds(
    env: &Environment,
    detection: Option<Detection>,
    require_foreground: bool,
) -> Option<ScreenRect> {
    if let Some(detection) = detection {
        if let Some(bounds) = screen_bounds(env, detection, require_foreground) {
            return Some(ScreenRect {
                x: bounds.x - f64::from(detection.rect.x),
                y: bounds.y - f64::from(detection.rect.y),
                width: bounds.width,
                height: bounds.height,
            });
        }
        if require_foreground {
            return None;
        }
    }
    let (window, pid, process_started_at) = find_game_window(env).ok()?;
    let fallback = Detection {
        window,
        pid,
        process_started_at,
        rect: PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    };
    let bounds = screen_bounds(env, fallback, require_foreground)?;
    Some(ScreenRect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    })
}

#[cfg(not(windows))]
fn game_client_bounds(
    _env: &Environment,
    _detection: Option<Detection>,
    _require_foreground: bool,
) -> Option<ScreenRect> {
    None
}

#[cfg(windows)]
fn screen_bounds(
    _env: &Environment,
    detection: Detection,
    require_foreground: bool,
) -> Option<ScreenRect> {
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
        || (require_foreground && unsafe { GetForegroundWindow() } as isize != detection.window)
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
fn screen_bounds(
    _env: &Environment,
    _detection: Detection,
    _require_foreground: bool,
) -> Option<ScreenRect> {
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

fn vertical_edge(
    frame: &Frame,
    first: u32,
    second: u32,
    top: u32,
    bottom: u32,
) -> Option<(u32, f64)> {
    (first.min(second).saturating_sub(4)
        ..=first
            .max(second)
            .saturating_add(4)
            .min(frame.width.saturating_sub(1)))
        .map(|x| (x, vertical_black_ratio(frame, x, top, bottom)))
        .filter(|(_, ratio)| *ratio >= 0.90)
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn vertical_black_span(frame: &Frame, x: u32, top: u32) -> Option<(u32, f64)> {
    let maximum_bottom = top.saturating_add(119).min(frame.height.saturating_sub(1));
    let mut black = 0u32;
    let mut last_black = None;
    let mut consecutive_gap = 0u32;
    for y in top..=maximum_bottom {
        if pixel(frame, x, y).is_some_and(|value| gray(value, 0, 2, 2)) {
            black += 1;
            last_black = Some(y);
            consecutive_gap = 0;
        } else {
            consecutive_gap += 1;
            if y >= top.saturating_add(20) && consecutive_gap >= 4 {
                break;
            }
        }
    }
    let bottom = last_black?;
    let total = bottom - top + 1;
    let ratio = f64::from(black) / f64::from(total);
    (total >= 20 && ratio >= 0.90).then_some((bottom, ratio))
}

fn vertical_span_near(frame: &Frame, expected: u32, top: u32) -> Option<(u32, u32, f64)> {
    (expected.saturating_sub(4)
        ..=expected
            .saturating_add(4)
            .min(frame.width.saturating_sub(1)))
        .filter_map(|x| {
            vertical_black_span(frame, x, top).map(|(bottom, ratio)| (x, bottom, ratio))
        })
        .max_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.2.total_cmp(&right.2))
        })
}

fn has_colored_health_fill(frame: &Frame, rect: PixelRect) -> bool {
    if rect.width < 5 || rect.height < 5 {
        return false;
    }
    let left = rect.x + 2;
    let right = rect.x + rect.width - 2;
    let required = (right - left).div_ceil(2);
    (rect.y + 2..rect.y + rect.height - 2).any(|y| {
        (left..right)
            .filter(|x| {
                pixel(frame, *x, y).is_some_and(|value| {
                    let low = *value.iter().min().unwrap();
                    let high = *value.iter().max().unwrap();
                    high >= 48 && high - low >= 16
                })
            })
            .count() as u32
            >= required
    })
}

fn detect_from_upper_edges(frame: &Frame) -> Option<PixelRect> {
    let mut candidates = Vec::new();
    for top in 0..frame.height.saturating_sub(19) {
        for upper in runs(frame, top, 0, 32, 150) {
            if upper.start < 5
                || upper.end.saturating_add(5) >= frame.width
                || upper.width() > frame.width / 2
            {
                continue;
            }
            let Some(left) = vertical_span_near(frame, upper.start, top) else {
                continue;
            };
            let Some(right) = vertical_span_near(frame, upper.end, top) else {
                continue;
            };
            if right.0 <= left.0 || left.1.abs_diff(right.1) > 4 {
                continue;
            }
            let width = right.0 - left.0 + 1;
            let bottom = left.1.max(right.1);
            let height = bottom - top + 1;
            if width < height.saturating_mul(5) {
                continue;
            }
            let rect = PixelRect {
                x: left.0,
                y: top,
                width,
                height,
            };
            if !has_colored_health_fill(frame, rect) {
                continue;
            }
            candidates.push((
                left.2 + right.2 + f64::from(width) / f64::from(frame.width),
                rect,
            ));
        }
    }
    candidates
        .into_iter()
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, rect)| rect)
}

fn detector_diagnostics(frame: &Frame) -> String {
    let mut lower = 0usize;
    let mut darkest = 0usize;
    let mut upper = 0usize;
    let mut grayscale = Vec::new();
    let mut minimum = u8::MAX;
    let mut maximum = 0u8;
    for y in 0..frame.height {
        lower += runs(frame, y, 53, 57, 150).len();
        darkest += runs(frame, y, 15, 19, 150).len();
        upper += runs(frame, y, 0, 32, 150).len();
        let mut start = None;
        let mut sum = 0u64;
        for x in 0..frame.width {
            let Some(value) = pixel(frame, x, y) else {
                continue;
            };
            let low = *value.iter().min().unwrap();
            let high = *value.iter().max().unwrap();
            minimum = minimum.min(low);
            maximum = maximum.max(high);
            if high <= 90 && high - low <= 3 {
                start.get_or_insert(x);
                sum += u64::from(value[0]) + u64::from(value[1]) + u64::from(value[2]);
            } else if let Some(run_start) = start.take() {
                let width = x - run_start;
                if width >= 150 {
                    grayscale.push((width, y, sum / u64::from(width * 3)));
                }
                sum = 0;
            }
        }
        if let Some(run_start) = start {
            let width = frame.width - run_start;
            if width >= 150 {
                grayscale.push((width, y, sum / u64::from(width * 3)));
            }
        }
    }
    grayscale.sort_by(|left, right| right.0.cmp(&left.0));
    let lines = grayscale
        .into_iter()
        .take(3)
        .map(|(width, y, value)| format!("{value}/{width}@{y}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut double_pairs = 0usize;
    let mut best_top: Option<(u32, u32, Run, Run, u32)> = None;
    for bottom in 0..frame.height.saturating_sub(1) {
        for lower_run in runs(frame, bottom, 53, 57, 150) {
            for dark_run in runs(frame, bottom + 1, 15, 19, 150) {
                if !close(lower_run.start, dark_run.start, 2)
                    || !close(lower_run.end, dark_run.end, 2)
                    || lower_run.width().abs_diff(dark_run.width()) > 2
                {
                    continue;
                }
                double_pairs += 1;
                let minimum_top = bottom.saturating_sub(120);
                let maximum_top = bottom.saturating_sub(20);
                for top in minimum_top..=maximum_top {
                    for upper_run in runs(frame, top, 0, 32, 150) {
                        let score = lower_run.start.abs_diff(upper_run.start)
                            + lower_run.end.abs_diff(upper_run.end)
                            + lower_run.width().abs_diff(upper_run.width());
                        if best_top
                            .as_ref()
                            .is_none_or(|(best, _, _, _, _)| score < *best)
                        {
                            best_top = Some((score, bottom, lower_run, upper_run, top));
                        }
                    }
                }
            }
        }
    }
    let top_detail = best_top.map_or_else(
        || "없음".to_owned(),
        |(score, bottom, lower_run, upper_run, top)| {
            let left = (lower_run.start.saturating_sub(2)
                ..=lower_run
                    .start
                    .saturating_add(2)
                    .min(frame.width.saturating_sub(1)))
                .map(|x| vertical_black_ratio(frame, x, top, bottom + 1))
                .fold(0.0, f64::max);
            let right = (lower_run.end.saturating_sub(2)
                ..=lower_run
                    .end
                    .saturating_add(2)
                    .min(frame.width.saturating_sub(1)))
                .map(|x| vertical_black_ratio(frame, x, top, bottom + 1))
                .fold(0.0, f64::max);
            format!(
                "차이{score}, 하단{}-{}@{bottom}, 상단{}-{}@{top}, 세로{:.0}%/{:.0}%",
                lower_run.start,
                lower_run.end,
                upper_run.start,
                upper_run.end,
                left * 100.0,
                right * 100.0
            )
        },
    );
    format!(
        "{}x{}, 밝기 {minimum}~{maximum}, 55선 {lower}, 17선 {darkest}, 이중선 {double_pairs}, 상단선0~32 {upper}, 최접 [{top_detail}], 회색후보 [{lines}]",
        frame.width, frame.height
    )
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
                    for upper in runs(frame, top, 0, 32, 150) {
                        let overlap_start = lower.start.max(upper.start);
                        let overlap_end = lower.end.min(upper.end);
                        if overlap_end < overlap_start || overlap_end - overlap_start + 1 < 150 {
                            continue;
                        }
                        let Some(left) =
                            vertical_edge(frame, lower.start, lower.start, top, bottom + 1)
                        else {
                            continue;
                        };
                        let Some(right) =
                            vertical_edge(frame, lower.end, lower.end, top, bottom + 1)
                        else {
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
                        let rect = PixelRect {
                            x: left.0,
                            y: top,
                            width,
                            height,
                        };
                        if !has_colored_health_fill(frame, rect) {
                            continue;
                        }
                        candidates.push((score, rect));
                    }
                }
            }
        }
    }
    candidates
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, rect)| rect)
        .or_else(|| detect_from_upper_edges(frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher_template() -> DebuffTemplate {
        let mut rgba = Vec::new();
        for value in 1..=9u8 {
            rgba.extend_from_slice(&[value, value + 10, value + 20, 255]);
        }
        DebuffTemplate {
            id: 1,
            kind: 1,
            name: "테스트".into(),
            width: 3,
            height: 3,
            rgba,
            anchor_x: 1,
            anchor_y: 1,
            anchor_rgb: [5, 15, 25],
            icon: String::new(),
        }
    }

    fn matcher_frame(template: &DebuffTemplate, scale: u32) -> Frame {
        let mut frame = Frame {
            width: 16,
            height: 12,
            bgra: vec![0; 16 * 12 * 4],
        };
        for ty in 0..template.height {
            for tx in 0..template.width {
                let source = (ty as usize * template.width as usize + tx as usize) * 4;
                for sy in 0..scale {
                    for sx in 0..scale {
                        let x = 4 + tx * scale + sx;
                        let y = 3 + ty * scale + sy;
                        let target = (y as usize * frame.width as usize + x as usize) * 4;
                        frame.bgra[target..target + 4].copy_from_slice(&[
                            template.rgba[source + 2],
                            template.rgba[source + 1],
                            template.rgba[source],
                            255,
                        ]);
                    }
                }
            }
        }
        frame
    }

    fn fixture(width: u32, height: u32, rect: PixelRect) -> Frame {
        let mut frame = Frame {
            width,
            height,
            bgra: vec![200; width as usize * height as usize * 4],
        };
        for pixel in frame.bgra.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        for y in rect.y + 1..rect.y + rect.height - 1 {
            for x in rect.x + 1..rect.x + rect.width - 1 {
                let index = (y as usize * width as usize + x as usize) * 4;
                frame.bgra[index..index + 3].copy_from_slice(&[90, 20, 120]);
            }
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
    fn 디버프_아이콘은_렌더러_반올림을_제외한_모든_rgb가_같을_때만_일치한다() {
        let template = matcher_template();
        let exact = matcher_frame(&template, 1);
        assert!(template_present(&exact, &template));

        let mut rounded = exact.clone();
        let index = (4usize * rounded.width as usize + 5) * 4;
        rounded.bgra[index] = rounded.bgra[index].saturating_add(2);
        assert!(template_present(&rounded, &template));

        let mut changed = exact.clone();
        let index = (4usize * changed.width as usize + 5) * 4;
        changed.bgra[index] = changed.bgra[index].saturating_add(3);
        assert!(!template_present(&changed, &template));
    }

    #[test]
    fn 디버프_아이콘의_두배_확대는_일치하지_않는다() {
        let template = matcher_template();
        assert!(!template_present(&matcher_frame(&template, 2), &template));
    }

    #[test]
    fn 배포_디버프_목록의_아이콘과_이름을_읽는다() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let templates = load_debuff_templates_from_root(&root).unwrap();
        assert_eq!(templates.len(), 16);
        assert_eq!(templates.first().map(|template| template.id), Some(1164));
        assert_eq!(
            templates.first().map(|template| template.name.as_str()),
            Some("물리보호")
        );
        assert_eq!(templates.last().map(|template| template.id), Some(16));
    }

    #[test]
    fn 디버프_판독_영역은_체력바_바로_위와_같은_크기다() {
        let health = PixelRect {
            x: 80,
            y: 80,
            width: 302,
            height: 38,
        };
        assert_eq!(
            debuff_source_rect(health),
            Some(PixelRect {
                x: 80,
                y: 42,
                width: 302,
                height: 38,
            })
        );
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
    fn 합성된_게임화면에서_밝아진_상단선을_허용한다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut frame = fixture(640, 180, expected);
        for x in expected.x + 1..expected.x + expected.width - 1 {
            let index = (expected.y as usize * frame.width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(31);
        }
        assert_eq!(detect_boss_health_bar(&frame), Some(expected));
    }

    #[test]
    fn 상단선_중앙_일부만_남아도_세로선으로_경계를_확정한다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut frame = fixture(640, 180, expected);
        let right = expected.x + expected.width - 1;
        for x in expected.x..expected.x + 20 {
            let index = (expected.y as usize * frame.width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(100);
        }
        for x in expected.x + 200..=right {
            let index = (expected.y as usize * frame.width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(100);
        }
        assert_eq!(detect_boss_health_bar(&frame), Some(expected));
    }

    #[test]
    fn 상하단_끝점_차이와_세로선_일부_간격을_허용한다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut frame = fixture(640, 180, expected);
        let bottom = expected.y + expected.height - 1;
        for y in [bottom - 1, bottom] {
            for x in expected.x + 1..=expected.x + 2 {
                let index = (y as usize * frame.width as usize + x as usize) * 4;
                frame.bgra[index..index + 3].fill(100);
            }
        }
        let right = expected.x + expected.width - 1;
        for x in [expected.x, right] {
            let index = (expected.y as usize * frame.width as usize + x as usize) * 4;
            frame.bgra[index..index + 3].fill(100);
        }
        for y in expected.y + 5..=expected.y + 6 {
            let index = (y as usize * frame.width as usize + right as usize) * 4;
            frame.bgra[index..index + 3].fill(100);
        }
        assert_eq!(
            detect_boss_health_bar(&frame),
            Some(expected),
            "{}",
            detector_diagnostics(&frame)
        );
    }

    #[test]
    fn 하단선_색상이_바뀌어도_상단과_세로선으로_찾는다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut frame = fixture(640, 180, expected);
        let right = expected.x + expected.width - 1;
        let bottom = expected.y + expected.height - 1;
        for y in [bottom - 1, bottom] {
            for x in expected.x + 1..right {
                let index = (y as usize * frame.width as usize + x as usize) * 4;
                frame.bgra[index..index + 3].copy_from_slice(&[120, 20, 90]);
            }
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
        for y in 1..39 {
            for x in 1..522 {
                set(x, y, 120, 20, 90);
            }
        }
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
    fn 하단선_변경은_허용하지만_검은_세로선이_없으면_거부한다() {
        let expected = PixelRect {
            x: 80,
            y: 40,
            width: 302,
            height: 38,
        };
        let mut no_double = fixture(640, 180, expected);
        let bottom = expected.y + expected.height - 1;
        for x in expected.x + 1..expected.x + expected.width - 1 {
            let index = (bottom as usize * no_double.width as usize + x as usize) * 4;
            no_double.bgra[index..index + 3].fill(80);
        }
        assert_eq!(detect_boss_health_bar(&no_double), Some(expected));

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
    fn 검은_배경의_직사각형_경계는_체력바로_인식하지_않는다() {
        let mut frame = Frame {
            width: 640,
            height: 180,
            bgra: vec![180; 640 * 180 * 4],
        };
        for pixel in frame.bgra.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        for x in 80..=381 {
            let index = (40usize * frame.width as usize + x) * 4;
            frame.bgra[index..index + 3].fill(0);
        }
        for y in 40..=77 {
            for x in [80usize, 381] {
                let index = (y * frame.width as usize + x) * 4;
                frame.bgra[index..index + 3].fill(0);
            }
        }

        assert_eq!(detect_boss_health_bar(&frame), None);
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

    #[test]
    fn 체력바_인식_없이도_디버프_알림_위치_구성을_시작한다() {
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

        let status = overlay.configure_alert(true).unwrap();

        assert_eq!(status["debuffRegionDetected"], false);
        assert_eq!(status["debuffAlertConfiguring"], true);
    }
}
