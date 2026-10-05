use crate::{
    Result,
    channel_ping::{OverlayBounds, clear_input_overlay_window, set_input_overlay_window},
    rpc::Rpc,
    storage,
    workers::{Environment, read},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Window {
    pub id: u64,
    pub kind: String,
    pub state: Value,
    pub loaded: bool,
    pub ready: bool,
    pub reveal: bool,
    pub closing: bool,
    pub animation: u64,
    pub drag: Option<(f64, f64, f64, f64)>,
}
struct State {
    windows: HashMap<u64, Window>,
    displays: Value,
    cursor: Value,
    preferred: Option<(f64, f64)>,
    page: String,
    tray_hidden: bool,
    revealed: bool,
    update_notified: Option<String>,
}
pub struct Windows {
    pub env: Environment,
    pub rpc: Rpc,
    state: Mutex<State>,
    sequence: AtomicU64,
    open_operation: Mutex<()>,
}
fn n(v: &Value, k: &str, default: f64) -> f64 {
    v[k].as_f64().filter(|n| n.is_finite()).unwrap_or(default)
}
impl Windows {
    pub fn new(env: Environment, rpc: Rpc, displays: Value) -> Self {
        Self {
            env,
            rpc,
            state: Mutex::new(State {
                windows: HashMap::new(),
                displays,
                cursor: Value::Null,
                preferred: None,
                page: "extract".into(),
                tray_hidden: false,
                revealed: false,
                update_notified: None,
            }),
            sequence: AtomicU64::new(2),
            open_operation: Mutex::new(()),
        }
    }
    pub fn kind(&self, id: u64) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .windows
            .get(&id)
            .map(|w| w.kind.clone())
    }
    pub fn get(&self, id: u64) -> Option<Window> {
        self.state.lock().unwrap().windows.get(&id).cloned()
    }
    pub fn id(&self, kind: &str) -> Option<u64> {
        self.state
            .lock()
            .unwrap()
            .windows
            .values()
            .find(|w| w.kind == kind)
            .map(|w| w.id)
    }
    pub fn channel_ping_visible(&self) -> bool {
        self.id("channel-ping-overlay")
            .and_then(|id| self.get(id))
            .is_some_and(|window| window.state["visible"] == true)
    }
    pub fn channel_ping_id(&self) -> Option<u64> {
        self.id("channel-ping-overlay")
    }
    pub fn channel_ping_live_visible(&self) -> bool {
        let state = self.state.lock().unwrap();
        let Some(window) = state
            .windows
            .values()
            .find(|window| window.kind == "channel-ping-live")
        else {
            return false;
        };
        #[cfg(windows)]
        {
            let Some(hwnd) = window.state["hwnd"].as_u64() else {
                return false;
            };
            return unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd as _) != 0
            };
        }
        #[cfg(not(windows))]
        {
            window.state["visible"] == true
        }
    }
    pub fn channel_ping_live_id(&self) -> Option<u64> {
        self.id("channel-ping-live")
    }
    pub fn arm_channel_ping_input(&self, id: u64, hwnd: isize) -> bool {
        let state = self.state.lock().unwrap();
        let valid = state.windows.get(&id).is_some_and(|window| {
            window.kind == "channel-ping-overlay"
                && window.state["hwnd"].as_u64() == Some(hwnd as u64)
        });
        if valid {
            set_input_overlay_window(Some((id, hwnd)));
        }
        valid
    }
    pub fn prepare_channel_ping(self: &Arc<Self>) -> Result<()> {
        self.open("channel-ping-overlay", false)?;
        self.open("channel-ping-live", false).map(|_| ())
    }
    pub fn show_channel_ping(self: &Arc<Self>, bounds: OverlayBounds) -> Result<(u64, isize)> {
        self.show_channel_ping_window("channel-ping-overlay", bounds, "전체 채널 상태 창")
    }
    pub fn show_channel_ping_live(self: &Arc<Self>, bounds: OverlayBounds) -> Result<(u64, isize)> {
        self.show_channel_ping_window("channel-ping-live", bounds, "상단 실시간 핑 창")
    }
    fn show_channel_ping_window(
        &self,
        kind: &str,
        bounds: OverlayBounds,
        label: &str,
    ) -> Result<(u64, isize)> {
        let id = self
            .id(kind)
            .ok_or_else(|| format!("{label}이 준비되지 않았습니다"))?;
        let hwnd = self
            .get(id)
            .and_then(|window| window.state["hwnd"].as_u64())
            .ok_or_else(|| format!("{label}의 native 창을 찾지 못했습니다"))?;
        self.command(
            id,
            "physicalBounds",
            json!({
                "x":bounds.x,
                "y":bounds.y,
                "width":bounds.width,
                "height":bounds.height
            }),
        );
        self.command(id, "alwaysOnTop", json!(true));
        self.command(id, "focusable", json!(false));
        self.command(id, "ignoreMouseEvents", json!(true));
        self.command(id, "opacity", json!(1));
        self.rpc.request(
            "window.command",
            json!({"windowId":id,"action":"showInactive","value":Value::Null}),
        )?;
        if let Some(window) = self.state.lock().unwrap().windows.get_mut(&id) {
            window.state["visible"] = json!(true);
        }
        if self.id(kind) != Some(id) {
            return Err(format!("{label}이 표시 중 다시 생성되었습니다"));
        }
        Ok((id, hwnd as isize))
    }
    pub fn hide_channel_ping(&self) {
        if let Some(id) = self.id("channel-ping-overlay") {
            self.command(id, "hide", Value::Null);
        }
    }
    pub fn hide_channel_ping_live(&self) {
        let id = {
            let mut state = self.state.lock().unwrap();
            let Some(window) = state
                .windows
                .values_mut()
                .find(|window| window.kind == "channel-ping-live")
            else {
                return;
            };
            #[cfg(windows)]
            if let Some(hwnd) = window.state["hwnd"].as_u64() {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindowAsync(
                        hwnd as _,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
                    );
                }
            }
            let id = window.id;
            window.state["visible"] = json!(false);
            id
        };
        if self.id("channel-ping-live") == Some(id) {
            self.command(id, "hide", Value::Null);
        }
    }
    pub fn destroy_channel_ping(&self) {
        for kind in ["channel-ping-overlay", "channel-ping-live"] {
            self.destroy_channel_ping_window(kind);
        }
    }
    fn destroy_channel_ping_window(&self, kind: &str) {
        let removed = {
            let mut state = self.state.lock().unwrap();
            let id = state
                .windows
                .values()
                .find(|window| window.kind == kind)
                .map(|window| window.id);
            id.and_then(|id| {
                clear_input_overlay_window(id);
                state.windows.remove(&id)
            })
        };
        let Some(window) = removed else { return };
        for action in ["hide", "destroy"] {
            self.rpc.native(
                "window.command",
                json!({"windowId":window.id,"action":action,"value":Value::Null}),
            );
        }
    }
    pub fn command(&self, id: u64, action: &str, value: Value) {
        {
            let mut state = self.state.lock().unwrap();
            let Some(window) = state.windows.get_mut(&id) else {
                return;
            };
            match action {
                "show" | "showInactive" => window.state["visible"] = json!(true),
                "hide" => window.state["visible"] = json!(false),
                "opacity" | "enabled" | "focusable" | "alwaysOnTop" => {
                    window.state[action] = value.clone()
                }
                "restore" => window.state["minimized"] = json!(false),
                "position" | "bounds" | "physicalBounds" => {
                    if let Some(fields) = value.as_object() {
                        for (k, v) in fields {
                            window.state[k] = v.clone();
                        }
                    }
                }
                _ => {}
            }
        }
        self.rpc.native(
            "window.command",
            json!({"windowId":id,"action":action,"value":value}),
        );
    }
    pub fn event(&self, id: u64, event: &str, new: &Value) {
        let mut state = self.state.lock().unwrap();
        let Some(window) = state.windows.get_mut(&id) else {
            return;
        };
        if let Some(fields) = new.as_object() {
            for (k, v) in fields {
                window.state[k] = v.clone();
            }
        }
        if event == "resize" && window.kind == "blackbox-manager" {
            let size = (
                n(&window.state, "width", 1040.).max(900.),
                n(&window.state, "height", 760.).max(680.),
            );
            if state.page == "extract" {
                state.preferred = Some(size);
                let _ = storage::write_json(
                    &self.env.user.join("blackbox/window.json"),
                    &json!({"width":size.0.round() as u64,"height":size.1.round() as u64,"updatedAt":crate::app_services::iso()}),
                );
            }
        }
    }
    pub fn displays(&self, value: Value, cursor: Value) {
        let mut state = self.state.lock().unwrap();
        state.displays = value;
        state.cursor = cursor;
    }
    pub fn visual_active(&self) -> bool {
        let state = self.state.lock().unwrap();
        state
            .windows
            .get(&1)
            .is_some_and(|w| w.state["visible"] == true && w.state["minimized"] != true)
            && state.windows.values().any(|w| w.state["focused"] == true)
    }
    pub fn notify_visual(&self) {
        self.rpc.event(
            1,
            "application:visual-activity-changed",
            json!([self.visual_active()]),
        );
    }
    fn work_area(&self, bounds: &Value) -> Value {
        let state = self.state.lock().unwrap();
        let displays = state.displays.as_array().cloned().unwrap_or_default();
        let cx = n(bounds, "x", 0.) + n(bounds, "width", 640.) / 2.;
        let cy = n(bounds, "y", 0.) + n(bounds, "height", 290.) / 2.;
        let display = displays.iter().max_by(|a, b| {
            let score = |v: &Value| {
                let r = &v["bounds"];
                let dx = (n(r, "x", 0.) - cx)
                    .max(0.)
                    .max(cx - n(r, "x", 0.) - n(r, "width", 1920.));
                let dy = (n(r, "y", 0.) - cy)
                    .max(0.)
                    .max(cy - n(r, "y", 0.) - n(r, "height", 1080.));
                -(dx * dx + dy * dy)
            };
            score(a).total_cmp(&score(b))
        });
        display
            .and_then(|d| d.get("workArea"))
            .cloned()
            .unwrap_or(json!({"x":0,"y":0,"width":1920,"height":1080}))
    }

    pub fn update_notice(self: &Arc<Self>, version: &str) -> Result<()> {
        let _opening = self.open_operation.lock().unwrap();
        if version.is_empty()
            || self.state.lock().unwrap().update_notified.as_deref() == Some(version)
        {
            return Ok(());
        }
        if let Some(id) = self.id("update-notice") {
            self.destroy(id);
        }
        let bounds = self.get(1).map(|w| w.state).unwrap_or_else(|| {
            let cursor = self.state.lock().unwrap().cursor.clone();
            json!({"x":cursor["x"],"y":cursor["y"],"width":0,"height":0})
        });
        let options = update_notice_options(&self.work_area(&bounds));
        let id = self.sequence.fetch_add(1, Ordering::SeqCst);
        self.state.lock().unwrap().windows.insert(
            id,
            Window {
                id,
                kind: "update-notice".into(),
                state: options.clone(),
                loaded: false,
                ready: true,
                reveal: false,
                closing: false,
                animation: 0,
                drag: None,
            },
        );
        let html = include_str!("update-notice.html");
        if let Err(e)=self.rpc.request("window.open",json!({"windowId":id,"file":format!("data:text/html;charset=utf-8,{}",urlencoding::encode(html)),"options":options})) {
            self.destroy(id);return Err(e);
        }
        self.command(id, "ignoreMouseEvents", json!(true));
        self.command(id, "allWorkspaces", json!(true));
        self.command(id, "showInactive", Value::Null);
        self.state.lock().unwrap().update_notified = Some(version.into());
        let this = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(4800));
            this.destroy(id);
        });
        Ok(())
    }

    pub fn notice(self: &Arc<Self>, value: &Value) -> Result<()> {
        let _opening = self.open_operation.lock().unwrap();
        let message = value["message"].as_str().unwrap_or("");
        let persistent = value["persistent"] == true;
        let existing = self.id("notice");
        let id = if let Some(id) = existing {
            self.rpc.request("window.eval",json!({"windowId":id,"script":format!("window.updateNotice?.({}, {})",json!(message),persistent)}))?;
            id
        } else {
            let cursor = self.state.lock().unwrap().cursor.clone();
            let bounds = if cursor["x"].is_number() {
                json!({"x":cursor["x"],"y":cursor["y"],"width":0,"height":0})
            } else {
                self.get(1).map(|w| w.state).unwrap_or(Value::Null)
            };
            let area = self.work_area(&bounds);
            let width = n(&area, "width", 1920.).min(480.);
            let id = self.sequence.fetch_add(1, Ordering::SeqCst);
            let escaped = message
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            let html = include_str!("blackbox-notice.html")
                .replace("__MESSAGE__", &escaped)
                .replace(
                    "__EXIT_TIMER__",
                    if persistent {
                        ""
                    } else {
                        "exitTimer=setTimeout(()=>notice.classList.add(\"exiting\"),2380)"
                    },
                );
            let options = json!({"x":n(&area,"x",0.)+n(&area,"width",1920.)-width-24.,"y":n(&area,"y",0.)+24.,"width":width,"height":88,"show":false,"opacity":1,"frame":false,"transparent":true,"resizable":false,"focusable":false,"skipTaskbar":true,"alwaysOnTop":true});
            self.state.lock().unwrap().windows.insert(
                id,
                Window {
                    id,
                    kind: "notice".into(),
                    state: options.clone(),
                    loaded: false,
                    ready: true,
                    reveal: false,
                    closing: false,
                    animation: 0,
                    drag: None,
                },
            );
            self.rpc.request("window.open",json!({"windowId":id,"file":format!("data:text/html;charset=utf-8,{}",urlencoding::encode(&html)),"options":options}))?;
            self.command(id, "ignoreMouseEvents", json!(true));
            self.command(id, "allWorkspaces", json!(true));
            self.command(id, "showInactive", Value::Null);
            id
        };
        let generation = {
            let mut state = self.state.lock().unwrap();
            let w = state.windows.get_mut(&id).ok_or("알림 창이 닫혔습니다")?;
            w.animation += 1;
            w.animation
        };
        if !persistent {
            let this = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(if existing.is_some() {
                    3300
                } else {
                    2800
                }));
                if this.get(id).is_some_and(|w| w.animation == generation) {
                    this.destroy(id);
                }
            });
        }
        Ok(())
    }
    pub fn open_main(self: &Arc<Self>, tray: bool) -> Result<()> {
        self.state.lock().unwrap().windows.insert(1,Window{id:1,kind:"main".into(),state:json!({"visible":false,"opacity":0,"focused":false,"width":640,"height":290}),loaded:false,ready:true,reveal:false,closing:false,animation:0,drag:None});
        self.rpc.request("window.open",json!({"windowId":1,"file":self.env.root.join("index.html"),"options":{"width":640,"height":290,"show":false,"opacity":0,"frame":false,"roundedCorners":false,"resizable":false,"maximizable":false}}))?;
        if let Some(w) = self.state.lock().unwrap().windows.get_mut(&1) {
            w.loaded = true;
        }
        if tray {
            self.command(1, "opacity", json!(1));
            self.command(1, "skipTaskbar", json!(true));
            self.state.lock().unwrap().tray_hidden = true;
        }
        Ok(())
    }
    pub fn tray(&self, enabled: bool) {
        self.rpc.native(
            "tray.create",
            json!({"icon":self.env.root.join("icon.ico")}),
        );
        self.tray_icon(enabled);
        self.rpc
            .native("tray.tooltip", json!({"text":"마비노기 렘 부스터"}));
        self.rpc.native("tray.menu",json!({"items":[{"label":"열기","enabled":true,"index":0},{"label":"종료","enabled":true,"index":1}]}));
    }
    pub fn tray_icon(&self, enabled: bool) {
        self.rpc.native(
            "tray.icon",
            json!({"icon":self.env.root.join(if enabled{"icon.ico"}else{"icon-paused.png"})}),
        );
    }
    pub fn focus_main(&self) {
        let hidden = self.state.lock().unwrap().tray_hidden;
        if hidden {
            self.destroy_aux();
        }
        for (action, value) in [
            ("restore", Value::Null),
            ("enabled", json!(true)),
            ("focusable", json!(true)),
            ("ignoreMouseEvents", json!(false)),
            ("alwaysOnTop", json!(false)),
            ("skipTaskbar", json!(false)),
            ("show", Value::Null),
            ("focus", Value::Null),
        ] {
            self.command(1, action, value);
        }
        self.state.lock().unwrap().tray_hidden = false;
        self.notify_visual();
    }
    pub fn minimize(&self) {
        self.destroy_aux();
        self.command(1, "hide", Value::Null);
        self.command(1, "skipTaskbar", json!(true));
        self.state.lock().unwrap().tray_hidden = true;
        self.notify_visual();
    }
    pub fn destroy(&self, id: u64) {
        if self
            .get(id)
            .is_some_and(|window| window.kind == "channel-ping-overlay")
        {
            self.destroy_channel_ping();
            return;
        }
        self.command(id, "hide", Value::Null);
        self.command(id, "destroy", Value::Null);
        self.state.lock().unwrap().windows.remove(&id);
    }
    pub fn destroy_aux(&self) {
        let ids: Vec<_> = self
            .state
            .lock()
            .unwrap()
            .windows
            .values()
            .filter(|w| w.id != 1 && !matches!(w.kind.as_str(), "notice" | "update-notice"))
            .map(|w| w.id)
            .collect();
        for id in ids {
            self.destroy(id);
        }
    }
    pub fn is_hidden(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.tray_hidden
            || state
                .windows
                .get(&1)
                .is_none_or(|w| w.state["visible"] != true)
    }
    pub fn reveal_main(self: &Arc<Self>) {
        let mut state = self.state.lock().unwrap();
        if state.revealed {
            return;
        }
        state.revealed = true;
        drop(state);
        self.command(1, "opacity", json!(0));
        self.focus_main();
        self.fade(1, 1., 1000, false, false);
    }
    pub fn fade(self: &Arc<Self>, id: u64, to: f64, duration: u64, close: bool, retain: bool) {
        let (from, generation) = {
            let mut state = self.state.lock().unwrap();
            let Some(w) = state.windows.get_mut(&id) else {
                return;
            };
            w.animation += 1;
            (n(&w.state, "opacity", 1.), w.animation)
        };
        let this = self.clone();
        std::thread::spawn(move || {
            let start = Instant::now();
            loop {
                if this.get(id).is_none_or(|w| w.animation != generation) {
                    return;
                }
                let p = (start.elapsed().as_secs_f64() * 1000. / duration as f64).min(1.);
                this.command(id, "opacity", json!(from + (to - from) * p));
                if p >= 1. {
                    break;
                }
                std::thread::sleep(Duration::from_millis(16));
            }
            if close {
                if retain {
                    this.command(id, "hide", Value::Null);
                    this.command(id, "focusable", json!(false));
                    this.command(id, "ignoreMouseEvents", json!(true));
                    if let Some(w) = this.state.lock().unwrap().windows.get_mut(&id) {
                        w.closing = false;
                        w.reveal = false;
                    }
                } else {
                    this.destroy(id);
                }
                this.focus_main();
            }
        });
    }
    pub fn content_ready(self: &Arc<Self>, id: u64) {
        let reveal = {
            let mut state = self.state.lock().unwrap();
            let Some(w) = state.windows.get_mut(&id) else {
                return;
            };
            w.ready = true;
            w.loaded && w.reveal
        };
        if reveal {
            self.reveal_aux(id);
        }
    }
    fn reveal_aux(self: &Arc<Self>, id: u64) {
        let Some(w) = self.get(id) else { return };
        if !w.loaded || !w.ready {
            return;
        }
        if w.state["visible"] == true {
            self.command(id, "focus", Value::Null);
            return;
        }
        for (a, v) in [
            ("restore", Value::Null),
            ("focusable", json!(true)),
            ("ignoreMouseEvents", json!(false)),
            ("center", Value::Null),
            ("show", Value::Null),
            ("focus", Value::Null),
        ] {
            self.command(id, a, v);
        }
        self.fade(
            id,
            if w.kind.ends_with("guide") { 0.9 } else { 1. },
            300,
            false,
            false,
        );
    }
    pub fn open(self: &Arc<Self>, kind: &str, reveal: bool) -> Result<u64> {
        let _opening = self.open_operation.lock().unwrap();
        if let Some(id) = self.id(kind) {
            if reveal {
                if let Some(w) = self.state.lock().unwrap().windows.get_mut(&id) {
                    w.reveal = true;
                    if w.closing {
                        w.animation += 1;
                        w.state["visible"] = json!(false);
                    }
                    w.closing = false;
                }
                self.reveal_aux(id);
            }
            return Ok(id);
        }
        let mut options = match kind {
            "character-guide" => {
                json!({"width":920,"height":900,"minWidth":520,"minHeight":360,"title":"주변 캐릭터 강제 간소화","transparent":true,"alwaysOnTop":true,"skipTaskbar":true})
            }
            "dxvk-guide" => {
                json!({"width":920,"height":900,"title":"DXVK Vulkan 설정 안내","transparent":true,"alwaysOnTop":true,"skipTaskbar":true,"resizable":false,"parent":1})
            }
            "dxvk-manager" => {
                json!({"width":560,"height":430,"title":"Vulkan 업데이트 관리","transparent":true,"skipTaskbar":true,"resizable":false,"parent":1})
            }
            "blackbox-manager" => {
                let saved = read(&self.env.user.join("blackbox/window.json"));
                let custom = saved["width"].as_u64().is_some_and(|n| n >= 900)
                    && saved["height"].as_u64().is_some_and(|n| n >= 680);
                let area = self.work_area(&self.get(1).map(|w| w.state).unwrap_or(Value::Null));
                let size = (
                    n(&saved, "width", 1040.)
                        .min(n(&area, "width", 1920.))
                        .max(900.),
                    n(&saved, "height", 760.)
                        .min(n(&area, "height", 1080.))
                        .max(680.),
                );
                {
                    let mut state = self.state.lock().unwrap();
                    state.preferred = custom.then_some(size);
                    state.page = "extract".into();
                }
                json!({"width":size.0,"height":size.1,"minWidth":900,"minHeight":680,"title":"게임 블랙박스 관리","parent":1,"skipTaskbar":true})
            }
            "blackbox-editor" => {
                json!({"width":1100,"height":720,"minWidth":780,"minHeight":560,"title":"블랙박스 영상 추출","alwaysOnTop":true})
            }
            "channel-ping-overlay" => {
                json!({"width":584,"height":400,"title":"채널별 서버 지연 추측","transparent":true,"alwaysOnTop":true,"skipTaskbar":true,"resizable":false,"focusable":false,"roundedCorners":true})
            }
            "channel-ping-live" => {
                json!({"width":520,"height":30,"title":"실시간 점수 확인","transparent":true,"alwaysOnTop":true,"skipTaskbar":true,"resizable":false,"focusable":false,"roundedCorners":false})
            }
            _ => return Err("허용되지 않은 보조 창".into()),
        };
        options["show"] = json!(false);
        options["opacity"] = json!(0);
        options["frame"] = json!(false);
        options["roundedCorners"] = json!(true);
        options["maximizable"] = json!(false);
        options["webPreferences"] =
            json!({"preload":self.env.root.join(format!("service/{kind}-preload.js"))});
        let id = self.sequence.fetch_add(1, Ordering::SeqCst);
        let mut state = options.clone();
        state["visible"] = json!(false);
        state["focused"] = json!(false);
        self.state.lock().unwrap().windows.insert(
            id,
            Window {
                id,
                kind: kind.into(),
                state,
                loaded: false,
                ready: kind != "dxvk-manager",
                reveal,
                closing: false,
                animation: 0,
                drag: None,
            },
        );
        let opened = match self.rpc.request(
            "window.open",
            json!({"windowId":id,"file":self.env.root.join(format!("{kind}.html")),"options":options}),
        ) {
            Ok(opened) => opened,
            Err(error) => {
                self.state.lock().unwrap().windows.remove(&id);
                return Err(error);
            }
        };
        if let Some(w) = self.state.lock().unwrap().windows.get_mut(&id) {
            w.loaded = true;
            if let Some(hwnd) = opened["hwnd"].as_u64() {
                w.state["hwnd"] = json!(hwnd);
            }
        }
        if reveal {
            self.reveal_aux(id);
        }
        Ok(id)
    }
    pub fn close(self: &Arc<Self>, id: u64) {
        let Some(w) = self.get(id) else { return };
        if w.closing {
            return;
        }
        if let Some(w) = self.state.lock().unwrap().windows.get_mut(&id) {
            w.closing = true;
        }
        if w.kind == "blackbox-editor" {
            self.destroy(id);
            self.focus_main();
        } else {
            self.fade(id, 0., 300, true, w.kind == "dxvk-manager");
        }
    }
    pub fn drag(&self, id: u64, action: &str, point: &Value) {
        let mut state = self.state.lock().unwrap();
        let Some(w) = state.windows.get_mut(&id) else {
            return;
        };
        if action == "drag-end" {
            w.drag = None;
            return;
        }
        let Some(x) = point["screenX"].as_f64().filter(|n| n.is_finite()) else {
            return;
        };
        let Some(y) = point["screenY"].as_f64().filter(|n| n.is_finite()) else {
            return;
        };
        if action == "drag-start" {
            w.drag = Some((x, y, n(&w.state, "x", 0.), n(&w.state, "y", 0.)));
        } else if action == "drag-move" {
            if let Some((px, py, wx, wy)) = w.drag {
                drop(state);
                self.command(
                    id,
                    "position",
                    json!({"x":(wx+x-px).round(),"y":(wy+y-py).round()}),
                );
            }
        } else {
            w.drag = None;
        }
    }
    fn resize_centered(&self, id: u64, width: f64, height: f64) {
        let Some(w) = self.get(id) else { return };
        let area = self.work_area(&w.state);
        let ax = n(&area, "x", 0.);
        let ay = n(&area, "y", 0.);
        let x = (n(&w.state, "x", 0.) - (width - n(&w.state, "width", width)) / 2.)
            .round()
            .min(ax + n(&area, "width", 1920.) - width)
            .max(ax);
        let y = (n(&w.state, "y", 0.) - (height - n(&w.state, "height", height)) / 2.)
            .round()
            .min(ay + n(&area, "height", 1080.) - height)
            .max(ay);
        self.command(
            id,
            "bounds",
            json!({"x":x,"y":y,"width":width,"height":height}),
        );
    }
    pub fn page(&self, id: u64, page: &str) -> Result<Value> {
        if !matches!(page, "extract" | "clips" | "settings") {
            return Err("허용되지 않은 블랙박스 관리 페이지입니다".into());
        }
        let w = self.get(id).ok_or("창이 닫혔습니다")?;
        let area = self.work_area(&w.state);
        let preferred = {
            let mut state = self.state.lock().unwrap();
            state.page = page.into();
            state.preferred.unwrap_or((1040., 760.))
        };
        let compact = page == "settings";
        self.command(
            id,
            "minimumSize",
            json!({"width":if compact{520}else{900},"height":if compact{720}else{680}}),
        );
        self.resize_centered(
            id,
            if compact { 540. } else { preferred.0.max(900.) }.min(n(&area, "width", 1920.)),
            if compact { 940. } else { preferred.1.max(680.) }.min(n(&area, "height", 1080.)),
        );
        Ok(json!(true))
    }
    pub fn fit_media(&self, id: u64, value: &Value) -> Result<Value> {
        let (mw, mh, vw, vh) = (
            n(value, "mediaWidth", 0.),
            n(value, "mediaHeight", 0.),
            n(value, "viewportWidth", 0.),
            n(value, "viewportHeight", 0.),
        );
        if mw <= 0. || mh <= 0. || vw < 100. || vh < 100. {
            return Ok(json!(false));
        }
        let w = self.get(id).ok_or("창이 닫혔습니다")?;
        let area = self.work_area(&w.state);
        let (aw, ah) = (n(&area, "width", 1920.), n(&area, "height", 1080.));
        let ratio = (mw / mh).clamp(0.5, 4.);
        let page = if value["page"] == "clips" {
            "clips"
        } else {
            "extract"
        };
        let preferred = {
            let mut state = self.state.lock().unwrap();
            state.page = page.into();
            state.preferred
        };
        let (bw, bh) = (n(&w.state, "width", 1040.), n(&w.state, "height", 760.));
        let (wo, ho) = ((bw - vw).max(0.), (bh - vh).max(0.));
        let (width, height) = if page == "clips" {
            ((wo + vh * ratio).round().min(aw).max(900.), bh.min(ah))
        } else if let Some((pw, ph)) = preferred {
            (pw.min(aw).max(900.), ph.min(ah).max(680.))
        } else {
            let width = bw.min(aw);
            let height = (ho + vw / ratio).round();
            if height > ah {
                (
                    width
                        .min((wo + (ah - ho).max(100.) * ratio).round())
                        .max(900.),
                    ah,
                )
            } else {
                (width, height)
            }
        };
        let height = height.min(ah).max(680.);
        if page == "extract" {
            self.state.lock().unwrap().preferred = Some((width, height));
            storage::write_json(
                &self.env.user.join("blackbox/window.json"),
                &json!({"width":width.round() as u64,"height":height.round() as u64,"updatedAt":crate::app_services::iso()}),
            )?;
        }
        self.resize_centered(id, width, height);
        Ok(json!(true))
    }
}

fn update_notice_options(area: &Value) -> Value {
    let width = (n(area, "width", 1920.) - 24.).min(680.);
    json!({"x":n(area,"x",0.)+n(area,"width",1920.)-width-24.,"y":n(area,"y",0.)+n(area,"height",1080.)-88.-24.,"width":width,"height":88,"show":false,"opacity":1,"frame":false,"transparent":true,"resizable":false,"movable":false,"focusable":false,"skipTaskbar":true,"alwaysOnTop":true,"hasShadow":false})
}
#[cfg(test)]
mod update_notice_tests {
    use super::*;
    #[test]
    fn bottom_right_respects_secondary_monitor_and_taskbar() {
        let o = update_notice_options(&json!({"x":-1920,"y":-200,"width":1920,"height":1040}));
        assert_eq!(o["x"], json!(-704.));
        assert_eq!(o["y"], json!(728.));
        assert_eq!(o["width"], json!(680.));
        assert_eq!(o["focusable"], false);
        assert_eq!(o["skipTaskbar"], true);
        let narrow = update_notice_options(&json!({"x":0,"y":0,"width":640,"height":480}));
        assert_eq!(narrow["width"], json!(616.));
    }
}
