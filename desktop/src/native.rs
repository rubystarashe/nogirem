use crate::bridge::Client;
use base64::Engine;
#[cfg(windows)]
use desktop::tao::platform::windows::{
    MonitorHandleExtWindows, WindowBuilderExtWindows, WindowExtWindows,
};
use desktop::trayicon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem},
};
use dioxus::{
    desktop::{
        self, Config, DesktopContext, WindowBuilder, WindowCloseBehaviour,
        tao::{
            dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
            event::{Event, WindowEvent},
        },
    },
    prelude::*,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone)]
pub struct Host(pub Rc<RefCell<HostState>>);
pub struct HostState {
    pub root: PathBuf,
    pub main: DesktopContext,
    pub windows: HashMap<u64, DesktopContext>,
    pub tray: Option<TrayIcon>,
    pub tray_menu: Option<Menu>,
}
#[derive(Clone)]
struct AuxiliaryContext {
    client: Client,
    window_id: u64,
    allowed: Vec<String>,
    ready: Rc<RefCell<Option<tokio::sync::oneshot::Sender<()>>>>,
}

pub fn displays(window: &DesktopContext) -> Value {
    let primary = window.primary_monitor();
    json!(window.available_monitors().map(|m| {
        let scale = m.scale_factor();
        let p = m.position().to_logical::<f64>(scale);
        let s = m.size().to_logical::<f64>(scale);
        let mut info = windows_sys::Win32::Graphics::Gdi::MONITORINFO { cbSize: std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32, ..Default::default() };
        let valid = unsafe { windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(m.hmonitor() as _, &mut info) } != 0;
        let work = if valid { json!({"x":info.rcWork.left as f64/scale,"y":info.rcWork.top as f64/scale,"width":(info.rcWork.right-info.rcWork.left) as f64/scale,"height":(info.rcWork.bottom-info.rcWork.top) as f64/scale}) }
            else { json!({"x":p.x,"y":p.y,"width":s.width,"height":s.height}) };
        json!({"primary":primary.as_ref() == Some(&m), "scaleFactor":scale,
            "bounds":{"x":p.x,"y":p.y,"width":s.width,"height":s.height},
            "workArea":work})
    }).collect::<Vec<_>>())
}

pub fn cursor(window: &DesktopContext) -> Value {
    let mut point = windows_sys::Win32::Foundation::POINT::default();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point);
    }
    let scale = window
        .available_monitors()
        .find(|m| {
            let p = m.position();
            let s = m.size();
            point.x >= p.x
                && point.y >= p.y
                && point.x < p.x + s.width as i32
                && point.y < p.y + s.height as i32
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    json!({"x":point.x as f64/scale,"y":point.y as f64/scale})
}

fn icon(path: &Path) -> Result<Icon, String> {
    let rgba = image::open(path).map_err(|e| e.to_string())?.into_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).map_err(|e| e.to_string())
}

fn window_icon() -> desktop::tao::window::Icon {
    let rgba = image::load_from_memory(include_bytes!("../../icon.ico")).expect("embedded app icon").into_rgba8();
    let (w, h) = rgba.dimensions();
    desktop::tao::window::Icon::from_rgba(rgba.into_raw(), w, h).expect("valid app icon")
}

pub fn config(root: &Path, title: &str, width: f64, height: f64) -> Config {
    let builder = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(width, height))
        .with_resizable(false)
        .with_maximizable(false)
        .with_decorations(false)
        .with_undecorated_shadow(false)
        .with_always_on_top(false)
        .with_visible(false)
        .with_skip_taskbar(std::env::args().any(|arg| arg == "--startup-tray"))
        .with_window_icon(Some(window_icon()))
        .with_taskbar_icon(Some(window_icon()));
    Config::new().with_window(builder).with_menu(None).with_resource_directory(root)
        .with_windows_browser_args("--autoplay-policy=no-user-gesture-required")
        .with_data_directory(std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Nogirem/WebView2"))
        .with_custom_index("<!doctype html><html lang=\"ko\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"></head><body><div id=\"main\"></div></body></html>".into())
        .with_tray_icon_show_window_on_click(false)
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        .with_exits_when_last_window_closes(true)
        .with_navigation_handler(|url| url.starts_with("http://dioxus.index") || url.starts_with("https://dioxus.index") || url == "about:blank")
}

pub fn asset_handlers() {
    for namespace in ["web", "public", "blackbox-editor.html"] {
        desktop::use_asset_handler(namespace, |request, responder| {
            let response = (|| {
                let path = urlencoding::decode(request.uri().path()).map_err(|e| e.to_string())?;
                let root = crate::root().canonicalize().map_err(|e| e.to_string())?;
                let file = root
                    .join(path.trim_start_matches('/'))
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !file.starts_with(root.join("web"))
                    && !file.starts_with(root.join("public"))
                    && file != root.join("blackbox-editor.html")
                {
                    return Err("Asset path outside permitted directories".into());
                }
                let content_type = match file.extension().and_then(|s| s.to_str()).unwrap_or("") {
                    "css" => "text/css",
                    "js" | "mjs" => "text/javascript",
                    "html" => "text/html",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "webp" => "image/webp",
                    "svg" => "image/svg+xml",
                    "woff" => "font/woff",
                    "mp3" => "audio/mpeg",
                    _ => return Err("Unsupported asset type".into()),
                };
                let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
                let length = bytes.len();
                let mut builder = desktop::wry::http::Response::builder()
                    .header("Content-Type", content_type)
                    .header("Accept-Ranges", "bytes");
                let mut body = bytes;
                if let Some(range) = request.headers().get("Range").and_then(|v| v.to_str().ok()) {
                    let range = range.strip_prefix("bytes=").and_then(|v| v.split_once('-'));
                    let bounds = range.and_then(|(start, end)| {
                        let start = if start.is_empty() {
                            length.saturating_sub(end.parse::<usize>().ok()?)
                        } else {
                            start.parse::<usize>().ok()?
                        };
                        let end = if end.is_empty() || range?.0.is_empty() {
                            length.checked_sub(1)?
                        } else {
                            end.parse::<usize>().ok()?.min(length.checked_sub(1)?)
                        };
                        (start <= end && start < length).then_some((start, end))
                    });
                    let Some((start, end)) = bounds else {
                        return Ok(builder
                            .status(416)
                            .header("Content-Range", format!("bytes */{length}"))
                            .body(Vec::new())
                            .unwrap());
                    };
                    builder = builder
                        .status(206)
                        .header("Content-Range", format!("bytes {start}-{end}/{length}"));
                    body = body[start..=end].to_vec();
                }
                builder = builder.header("Content-Length", body.len().to_string());
                if request.method() == desktop::wry::http::Method::HEAD {
                    body.clear();
                }
                Ok(builder.body(body).unwrap())
            })();
            responder.respond(response.unwrap_or_else(|error: String| {
                desktop::wry::http::Response::builder()
                    .status(404)
                    .body(error.into_bytes())
                    .unwrap()
            }));
        });
    }
}

pub fn window_events(client: Client, window_id: u64) {
    let window = desktop::use_window();
    desktop::use_wry_event_handler(move |event, _| {
        if let Event::WindowEvent {
            window_id: id,
            event,
            ..
        } = event
        {
            if *id != window.id() {
                return;
            }
            let (name, state) = match event {
                WindowEvent::CloseRequested => {
                    // Closing is decided by the backend (save guard / restore prompt).
                    let window = window.clone();
                    let client = client.clone();
                    spawn(async move {
                        tokio::task::yield_now().await;
                        window.set_visible(true);
                        client.notify("window", json!({"windowId":window_id,"event":"close"}));
                    });
                    return;
                }
                WindowEvent::Focused(focused) => (
                    if *focused { "focus" } else { "blur" },
                    json!({"focused":focused}),
                ),
                WindowEvent::Moved(position) => {
                    let p = position.to_logical::<f64>(window.scale_factor());
                    ("move", json!({"x":p.x,"y":p.y}))
                }
                WindowEvent::Resized(size) => {
                    let s = size.to_logical::<f64>(window.scale_factor());
                    ("resize", json!({"width":s.width,"height":s.height}))
                }
                _ => return,
            };
            client.notify(
                "window",
                json!({"windowId":window_id,"event":name,"state":state}),
            );
        }
    });
}

fn auxiliary() -> Element {
    asset_handlers();
    let ctx = use_context::<AuxiliaryContext>();
    let window = desktop::use_window();
    use_hook({
        let window = window.clone();
        move || window.set_zoom_level(1.0)
    });
    window_events(ctx.client.clone(), ctx.window_id);
    use_future({
        let ctx = ctx.clone();
        move || {
            let ctx = ctx.clone();
            async move {
                // Resolve loadFile only after Dioxus has initialized and applied its visibility.
                let _ = document::eval(
                    "await new Promise(resolve => requestAnimationFrame(resolve)); return true",
                )
                .await;
                if let Some(ready) = ctx.ready.borrow_mut().take() {
                    let _ = ready.send(());
                }
            }
        }
    });
    use_future(move || {
        let ctx = ctx.clone();
        let window = window.clone();
        async move {
            let mut eval = document::eval(
                r#"
                window.__nogiremAttach(message => dioxus.send(message));
                const {attachWindowEscape} = await import('/web/window-escape.js');
                attachWindowEscape(message => dioxus.send(message));
                while (true) window.__nogiremReply(await dioxus.recv());
            "#,
            );
            while let Ok(message) = eval.recv::<Value>().await {
                if message["drag"] == true {
                    desktop::window().drag();
                    continue;
                }
                if message["close"] == true {
                    ctx.client
                        .notify("window", json!({"windowId":ctx.window_id,"event":"close"}));
                    continue;
                }
                if let Some(url) = message["external"].as_str() {
                    let _ =
                        crate::platform::dispatch("shell.openExternal", &json!({"path":url}), None)
                            .await;
                    continue;
                }
                if message["key"] == "Escape" {
                    ctx.client.notify("window", json!({"windowId":ctx.window_id,"event":"key","state":{"key":"Escape","type":"keyDown","isAutoRepeat":false}}));
                    continue;
                }
                let ctx = ctx.clone();
                let window = window.clone();
                spawn(async move {
                    let channel = message["channel"].as_str().unwrap_or("");
                    let reply = if !ctx.allowed.iter().any(|allowed| allowed == channel) {
                        Err("이 창에 허용되지 않은 요청입니다".into())
                    } else if channel == "character-guide:drag-start"
                        || channel == "dxvk-guide:drag-start"
                    {
                        window.drag();
                        Ok(Value::Null)
                    } else if channel == "character-guide:drag-move"
                        || channel == "character-guide:drag-end"
                        || channel == "dxvk-guide:drag-move"
                        || channel == "dxvk-guide:drag-end"
                    {
                        Ok(Value::Null)
                    } else if message["notify"] == true {
                        ctx.client.notify("send", json!({"windowId":ctx.window_id,"channel":channel,"args":message["args"]}));
                        Ok(Value::Null)
                    } else {
                        ctx.client
                            .invoke(ctx.window_id, channel, message["args"].clone())
                            .await
                    };
                    let response = match reply {
                        Ok(result) => json!({"id":message["id"],"result":result}),
                        Err(error) => json!({"id":message["id"],"error":error}),
                    };
                    let _ = window
                        .webview
                        .evaluate_script(&format!("window.__nogiremReply({response})"));
                });
            }
        }
    });
    rsx! {}
}

fn allowed_channels(source: &str) -> Vec<String> {
    ["ipcRenderer.invoke(\"", "ipcRenderer.send(\""]
        .iter()
        .flat_map(|prefix| {
            source
                .split(prefix)
                .skip(1)
                .filter_map(|part| part.split('"').next())
                .map(str::to_owned)
        })
        .collect()
}

impl Host {
    pub async fn dispatch(
        &self,
        client: &Client,
        method: &str,
        params: &Value,
    ) -> Result<Value, String> {
        match method {
            "backend.cpu-allocation" => {
                return serde_json::to_value(nogirem_backend::topology::resolve(
                    params["gameCoreCount"].as_i64(),
                )?)
                .map_err(|e| e.to_string());
            }
            "backend.memory-status" => {
                let config =
                    nogirem_backend::storage::read_json(&self.0.borrow().root.join("config.json"))?
                        .ok_or("config.json missing")?;
                let current = nogirem_backend::memory::snapshot()?;
                let cleaner =
                    nogirem_backend::memory::Cleaner::new(&config["memoryCleaner"], current.total);
                let mut result = cleaner.status(current);
                result["running"] = json!(false);
                return Ok(result);
            }
            "window.open" => {
                let id = params["windowId"].as_u64().ok_or("Missing window id")?;
                if id == 1 {
                    let main = self.0.borrow().main.clone();
                    crate::platform::configure_frame(
                        &main,
                        params["options"]["roundedCorners"]
                            .as_bool()
                            .unwrap_or(true),
                    );
                    crate::platform::set_opacity(
                        &main,
                        params["options"]["opacity"].as_f64().unwrap_or(0.0),
                    );
                    if let Some(monitor) = main.primary_monitor() {
                        let p = monitor.position();
                        let s = monitor.size();
                        let size = main.outer_size();
                        main.set_outer_position(desktop::tao::dpi::PhysicalPosition::new(
                            p.x + (s.width as i32 - size.width as i32) / 2,
                            p.y + (s.height as i32 - size.height as i32) / 2,
                        ));
                    }
                    client.notify("window", json!({"windowId":id,"event":"focus","state":{"focused":main.is_focused(),"visible":main.is_visible()}}));
                    self.0.borrow_mut().windows.insert(id, main);
                    return Ok(Value::Null);
                }
                let root = self.0.borrow().root.clone();
                let file = params["file"].as_str().ok_or("Missing document")?;
                let options = &params["options"];
                let mut html = if let Some(data) = file
                    .strip_prefix("data:text/html;charset=utf-8,")
                    .or_else(|| file.strip_prefix("data:text/html,"))
                {
                    urlencoding::decode(data)
                        .map_err(|e| e.to_string())?
                        .into_owned()
                } else {
                    let requested = PathBuf::from(file);
                    let name = requested.file_name().ok_or("Invalid document")?;
                    let path = root.join(name);
                    if ![
                        "character-guide.html",
                        "dxvk-manager.html",
                        "dxvk-guide.html",
                        "blackbox-manager.html",
                        "blackbox-editor.html",
                        "channel-ping-overlay.html",
                    ]
                    .iter()
                    .any(|n| name == *n)
                    {
                        return Err("Unknown auxiliary document".into());
                    }
                    std::fs::read_to_string(path).map_err(|e| e.to_string())?
                };
                for name in ["logo2-white-transparent.png", "doc_minimize/", "doc_dxvk/"] {
                    html = html.replace(&format!("./{name}"), &format!("./public/{name}"));
                }
                let preload = options["webPreferences"]["preload"]
                    .as_str()
                    .map(|p| {
                        let name = Path::new(p).file_name().ok_or("Invalid preload")?;
                        std::fs::read_to_string(root.join("service").join(name))
                            .map_err(|e| e.to_string())
                    })
                    .transpose()?
                    .unwrap_or_default();
                let allowed = allowed_channels(&preload);
                let bridge = include_str!("browser-bridge.js");
                let injection = format!("<script>{bridge}\n(() => {{ {preload} }})();</script>");
                if let Some(index) = html.find("<head>") {
                    html.insert_str(index + 6, &injection);
                } else {
                    html = format!("{injection}{html}");
                }
                html = html.replace("</body>", "<div id=\"dioxus-auxiliary-root\"></div></body>");
                if !html.contains("dioxus-auxiliary-root") {
                    html.push_str("<div id=\"dioxus-auxiliary-root\"></div>");
                }
                let width = options["width"].as_f64().unwrap_or(640.0);
                let height = options["height"].as_f64().unwrap_or(430.0);
                let mut builder = WindowBuilder::new()
                    .with_window_icon(Some(window_icon()))
                    .with_taskbar_icon(Some(window_icon()))
                    .with_title(options["title"].as_str().unwrap_or("마비노기 렘 부스터"))
                    .with_inner_size(LogicalSize::new(width, height))
                    .with_decorations(options["frame"].as_bool().unwrap_or(false))
                    .with_undecorated_shadow(false)
                    .with_resizable(options["resizable"].as_bool().unwrap_or(true))
                    .with_maximizable(options["maximizable"].as_bool().unwrap_or(true))
                    .with_focusable(options["focusable"].as_bool().unwrap_or(true))
                    .with_focused(options["focusable"].as_bool().unwrap_or(true))
                    .with_skip_taskbar(options["skipTaskbar"].as_bool().unwrap_or(false))
                    .with_min_inner_size(LogicalSize::new(
                        options["minWidth"].as_f64().unwrap_or(0.0),
                        options["minHeight"].as_f64().unwrap_or(0.0),
                    ))
                    .with_visible(false)
                    .with_transparent(options["transparent"].as_bool().unwrap_or(false))
                    .with_always_on_top(options["alwaysOnTop"].as_bool().unwrap_or(false));
                if let (Some(x), Some(y)) = (options["x"].as_f64(), options["y"].as_f64()) {
                    builder = builder.with_position(LogicalPosition::new(x, y));
                }
                if let Some(parent) = options["parent"]
                    .as_u64()
                    .and_then(|id| self.0.borrow().windows.get(&id).cloned())
                {
                    builder = builder.with_owner_window(parent.hwnd());
                }
                let media_client = client.clone();
                let runtime = tokio::runtime::Handle::current();
                let cfg = config(&root, "", width, height).with_window(builder).with_custom_index(html)
                    .with_background_color(if options["transparent"] == true { (0,0,0,0) } else { (255,255,255,255) })
                    .with_asynchronous_custom_protocol("nogirem-blackbox", move |_, request, responder| {
                        let client = media_client.clone();
                        runtime.spawn(async move {
                            let headers = request.headers().iter().filter_map(|(name, value)| value.to_str().ok().map(|value| (name.to_string(), value.to_owned()))).collect::<HashMap<_,_>>();
                            let result = client.request("protocol", json!({"url":request.uri().to_string(),"method":request.method().as_str(),"headers":headers})).await;
                            let response = match result {
                                Ok(value) => {
                                    let mut builder = desktop::wry::http::Response::builder().status(value["status"].as_u64().unwrap_or(500) as u16);
                                    if let Some(headers) = value["headers"].as_object() { for (name, value) in headers { if let Some(value) = value.as_str() { builder = builder.header(name, value); } } }
                                    let bytes = base64::engine::general_purpose::STANDARD.decode(value["body"].as_str().unwrap_or("")).unwrap_or_default();
                                    builder.body(bytes).unwrap()
                                },
                                Err(error) => desktop::wry::http::Response::builder().status(500).body(error.into_bytes()).unwrap(),
                            };
                            responder.respond(response);
                        });
                    })
                    .with_root_name("dioxus-auxiliary-root");
                let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();
                let dom = VirtualDom::new(auxiliary);
                dom.provide_root_context(AuxiliaryContext {
                    client: client.clone(),
                    window_id: id,
                    allowed,
                    ready: Rc::new(RefCell::new(Some(ready_sender))),
                });
                let main = self.0.borrow().main.clone();
                let window = main.new_window(dom, cfg).await;
                crate::platform::configure_frame(
                    &window,
                    options["roundedCorners"].as_bool().unwrap_or(true),
                );
                if options["x"].as_f64().is_none() || options["y"].as_f64().is_none() {
                    if let Some(monitor) = main.current_monitor().or_else(|| main.primary_monitor())
                    {
                        center_on_monitor(&window, &monitor);
                    }
                }
                crate::platform::set_opacity(&window, options["opacity"].as_f64().unwrap_or(1.0));
                self.0.borrow_mut().windows.insert(id, window.clone());
                tokio::time::timeout(std::time::Duration::from_secs(20), ready_receiver)
                    .await
                    .map_err(|_| "Auxiliary WebView initialization timed out")?
                    .map_err(|_| "Auxiliary WebView initialization canceled")?;
                crate::platform::configure_frame(
                    &window,
                    options["roundedCorners"].as_bool().unwrap_or(true),
                );
                let p = window
                    .outer_position()
                    .map_err(|e| e.to_string())?
                    .to_logical::<f64>(window.scale_factor());
                let size = window.inner_size().to_logical::<f64>(window.scale_factor());
                client.notify("window",json!({"windowId":id,"event":"move","state":{"x":p.x,"y":p.y,"width":size.width,"height":size.height}}));
                window.set_visible(options["show"] != false);
            }
            "window.command" => {
                let id = params["windowId"].as_u64().ok_or("Missing window id")?;
                let window = self
                    .0
                    .borrow()
                    .windows
                    .get(&id)
                    .cloned()
                    .ok_or("Unknown window")?;
                let value = &params["value"];
                let enabled = value.as_bool().unwrap_or(false);
                match params["action"].as_str().unwrap_or("") {
                    "show" => {
                        window.set_visible(true);
                        window.set_focus();
                    }
                    "showInactive" => crate::platform::show_inactive(&window),
                    "hide" => window.set_visible(false),
                    "focus" | "moveTop" => window.set_focus(),
                    "restore" => window.set_minimized(false),
                    "alwaysOnTop" => window.set_always_on_top(enabled),
                    "enabled" => window.set_enable(enabled),
                    "focusable" => window.set_focusable(enabled),
                    "ignoreMouseEvents" => window
                        .set_ignore_cursor_events(enabled)
                        .map_err(|e| e.to_string())?,
                    "position" => window.set_outer_position(LogicalPosition::new(
                        value["x"].as_f64().unwrap_or(0.0),
                        value["y"].as_f64().unwrap_or(0.0),
                    )),
                    "bounds" => {
                        window.set_outer_position(LogicalPosition::new(
                            value["x"].as_f64().unwrap_or(0.0),
                            value["y"].as_f64().unwrap_or(0.0),
                        ));
                        window.set_inner_size(LogicalSize::new(
                            value["width"].as_f64().unwrap_or(640.0),
                            value["height"].as_f64().unwrap_or(290.0),
                        ));
                    }
                    "physicalBounds" => {
                        window.set_outer_position(PhysicalPosition::new(
                            value["x"].as_f64().unwrap_or(0.0) as i32,
                            value["y"].as_f64().unwrap_or(0.0) as i32,
                        ));
                        window.set_inner_size(PhysicalSize::new(
                            value["width"].as_f64().unwrap_or(640.0).max(1.0) as u32,
                            value["height"].as_f64().unwrap_or(290.0).max(1.0) as u32,
                        ));
                    }
                    "minimumSize" => window.set_min_inner_size(Some(LogicalSize::new(
                        value["width"].as_f64().unwrap_or(0.0),
                        value["height"].as_f64().unwrap_or(0.0),
                    ))),
                    "center" => {
                        if let Some(monitor) = window.current_monitor() {
                            center_on_monitor(&window, &monitor);
                        }
                    }
                    "skipTaskbar" => {
                        #[cfg(windows)]
                        {
                            use desktop::tao::platform::windows::WindowExtWindows;
                            window
                                .set_skip_taskbar(enabled)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    "destroy" => {
                        crate::platform::hide_for_close(&window);
                        self.0.borrow_mut().windows.remove(&id);
                        window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
                        window.close();
                    }
                    "allWorkspaces" => {} // Windows has no workspace API corresponding to this macOS flag.
                    "opacity" => {
                        crate::platform::set_opacity(&window, value.as_f64().unwrap_or(1.0))
                    }
                    action => return Err(format!("Unsupported native window action: {action}")),
                }
            }
            "window.event" => {
                if let Some(window) = self
                    .0
                    .borrow()
                    .windows
                    .get(&params["windowId"].as_u64().unwrap_or(0))
                {
                    window
                        .webview
                        .evaluate_script(&format!(
                            "window.__nogiremDispatch?.({}, {})",
                            params["channel"], params["args"]
                        ))
                        .map_err(|e| e.to_string())?;
                }
            }
            "window.eval" => {
                let window = self
                    .0
                    .borrow()
                    .windows
                    .get(&params["windowId"].as_u64().unwrap_or(0))
                    .cloned()
                    .ok_or("Unknown window")?;
                window
                    .webview
                    .evaluate_script(params["script"].as_str().ok_or("Missing script")?)
                    .map_err(|e| e.to_string())?;
            }
            "window.background-throttling" => {} // Native service polling is independent of WebView visibility.
            "tray.create" => {
                let path = params["icon"].as_str().ok_or("Missing tray icon")?;
                let tray = TrayIconBuilder::new()
                    .with_icon(icon(Path::new(path))?)
                    .with_menu_on_left_click(false)
                    .build()
                    .map_err(|e| e.to_string())?;
                self.0.borrow_mut().tray = Some(tray);
            }
            "tray.icon" => {
                if let Some(tray) = &self.0.borrow().tray {
                    tray.set_icon(Some(icon(Path::new(
                        params["icon"].as_str().ok_or("Missing icon")?,
                    ))?))
                    .map_err(|e| e.to_string())?;
                }
            }
            "tray.tooltip" => {
                if let Some(tray) = &self.0.borrow().tray {
                    tray.set_tooltip(params["text"].as_str())
                        .map_err(|e| e.to_string())?;
                }
            }
            "tray.menu" => {
                let menu = Menu::new();
                for item in params["items"].as_array().ok_or("Missing tray items")? {
                    menu.append(&MenuItem::with_id(
                        format!("nogirem-{}", item["index"]),
                        item["label"].as_str().unwrap_or(""),
                        item["enabled"].as_bool().unwrap_or(true),
                        None,
                    ))
                    .map_err(|e| e.to_string())?;
                }
                if let Some(tray) = &self.0.borrow().tray {
                    tray.set_menu(Some(Box::new(menu.clone())));
                }
                self.0.borrow_mut().tray_menu = Some(menu);
            }
            "tray.destroy" => {
                self.0.borrow_mut().tray = None;
                self.0.borrow_mut().tray_menu = None;
            }
            "application.exit" => {
                let mut host = self.0.borrow_mut();
                host.tray = None;
                host.tray_menu = None;
                // Hide every surface before any webview is torn down (including the main
                // window when it has not yet been registered in the window map).
                crate::platform::hide_for_close(&host.main);
                for (&id, window) in &host.windows {
                    if id != 1 {
                        crate::platform::hide_for_close(window);
                    }
                }
                for (id, window) in host.windows.drain() {
                    if id == 1 {
                        continue;
                    }
                    window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
                    window.close();
                }
                host.main
                    .set_close_behavior(WindowCloseBehaviour::WindowCloses);
                host.main.close();
            }
            _ => {
                let parent = self
                    .0
                    .borrow()
                    .windows
                    .get(&params["windowId"].as_u64().unwrap_or(1))
                    .cloned();
                return crate::platform::dispatch(method, params, parent).await;
            }
        }
        Ok(Value::Null)
    }
}

fn center_on_monitor(window: &DesktopContext, monitor: &desktop::tao::monitor::MonitorHandle) {
    // Win32 monitor/work rectangles are physical. Keep their origins in that coordinate space,
    // including monitors left of the primary and monitors with different DPI scaling.
    let mut info = windows_sys::Win32::Graphics::Gdi::MONITORINFO {
        cbSize: std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32,
        ..Default::default()
    };
    let p = monitor.position();
    let s = monitor.size();
    let (x, y, w, h) = if unsafe {
        windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(monitor.hmonitor() as _, &mut info)
    } != 0
    {
        (
            info.rcWork.left,
            info.rcWork.top,
            info.rcWork.right - info.rcWork.left,
            info.rcWork.bottom - info.rcWork.top,
        )
    } else {
        (p.x, p.y, s.width as i32, s.height as i32)
    };
    let size = window.outer_size();
    window.set_outer_position(desktop::tao::dpi::PhysicalPosition::new(
        x + (w - size.width as i32) / 2,
        y + (h - size.height as i32) / 2,
    ));
}
