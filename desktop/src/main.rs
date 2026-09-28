#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod bridge;
mod markdown;
mod native;
mod platform;
mod smoke;
mod ui;
mod wave;

use bridge::Client;
use dioxus::{
    desktop::{
        self,
        trayicon::{MouseButton, MouseButtonState, TrayIconEvent},
    },
    prelude::*,
};
use serde_json::json;
use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

fn root() -> PathBuf {
    std::env::var_os("NOGIREM_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe().ok().and_then(|exe| {
                exe.parent()
                    .filter(|path| path.join("config.json").is_file())
                    .map(PathBuf::from)
            })
        })
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_owned()
        })
}

fn main() {
    // Native workers have no WebView, Node runtime, or application instance lock.
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--apply-update" || a == "--migrate-electron") {
        match platform::ensure_elevated() {
            Ok(true) => {}, Ok(false) => return,
            Err(error) => {rfd::MessageDialog::new().set_title("업데이트 권한 오류").set_description(error).show();return;}
        }
        let result = if args[i] == "--apply-update" {
            args.get(i+1).ok_or_else(|| "업데이트 작업 경로 누락".to_owned()).and_then(|p|nogirem_backend::update_install::apply(std::path::Path::new(p)))
        } else {
            (|| { let exe=args.get(i+1).ok_or("기존 실행 경로 누락")?; let pid=args.get(i+2).ok_or("기존 프로세스 누락")?.parse::<u32>().map_err(|e|e.to_string())?;nogirem_backend::update_install::migrate(std::path::Path::new(exe),pid) })()
        };
        if let Err(error) = result {
            if args[i]=="--apply-update" {if let Some(p)=args.get(i+1){nogirem_backend::update_install::recover_preinstall(std::path::Path::new(p));}}
            if args[i]=="--apply-update" { if let Some(p)=args.get(i+1) { let _=std::fs::write(std::path::Path::new(p).join("failure.txt"), &error); } }
            eprintln!("Update failed: {error}");
            rfd::MessageDialog::new().set_title("업데이트를 완료하지 못했습니다").set_description(&error).show();
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a|a=="--native-service") {
        if let Err(error)=nogirem_backend::service::run(&root(),&args){eprintln!("Rust service failed: {error}");std::process::exit(1);}
        return;
    }
    if let Some(index) = args.iter().position(|a| a == "--native-operation") {
        let result = (|| -> Result<serde_json::Value, String> {
            let action = args.get(index + 1).ok_or("Native operation missing")?;
            let params: serde_json::Value =
                serde_json::from_str(args.get(index + 2).ok_or("Native parameters missing")?)
                    .map_err(|e| e.to_string())?;
            if action == "graphics-check" || action == "graphics-apply" {
                nogirem_backend::graphics::run(&root(),params["gameExecutable"].as_str().unwrap_or(""),action == "graphics-apply")
            } else if action.starts_with("app-") {
                nogirem_backend::app_services::dispatch(&root(),action,&params)
            } else if action == "muo-status" {
                nogirem_backend::muo::latest(std::path::Path::new(params["directory"].as_str().ok_or("Missing directory")?))
            } else if action == "muo-install" {
                nogirem_backend::muo::install(&root(),std::path::Path::new(params["documents"].as_str().ok_or("Missing documents")?),params["fileName"].as_str().ok_or("Missing file name")?)
            } else if action.starts_with("dxvk-") {
                nogirem_backend::dxvk::dispatch(action,&params)
            } else {
                nogirem_backend::network::dispatch(action, &params)
            }
        })();
        match result {
            Ok(value) => println!("{}", json!({"ok":true,"data":value})),
            Err(error) => {
                println!("{}", json!({"ok":false,"error":error}));
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--native-affinity-helper") {
        if let Err(error) = nogirem_backend::affinity_worker::run(&root(), &args) {
            eprintln!("Affinity worker failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--native-memory-helper") {
        if let Err(error) = nogirem_backend::memory::run(&root(), &args) {
            eprintln!("Memory worker failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let instance = match platform::acquire_instance() {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    platform::retain_instance(instance);
    if !args.iter().any(|a|a=="--smoke-test") {
        match platform::ensure_elevated() {
            Ok(true)=>{}, Ok(false)=>return,
            Err(error)=>{rfd::MessageDialog::new().set_title("마비노기 렘 부스터").set_description(format!("관리자 권한으로 시작하지 못했습니다: {error}")).show();return;}
        }
    }
    let cfg = native::config(&root(), "마비노기 렘 부스터", 640.0, 290.0);
    LaunchBuilder::desktop().with_cfg(cfg).launch(app);
}

fn app() -> Element {
    native::asset_handlers();
    let window = desktop::use_window();
    let root = root();
    let connection = use_hook({
        let window = window.clone();
        let root = root.clone();
        move || {
            Client::start(&root, native::displays(&window))
                .map(|(client, receiver)| (client, Rc::new(RefCell::new(Some(receiver)))))
        }
    });
    let Ok((client, incoming)) = connection else {
        return rsx! { main { h1 { "시작 실패" } p { "{connection.as_ref().err().unwrap()}" } } };
    };
    use_context_provider(|| client.clone());
    let mut state = use_context_provider(|| Signal::new(ui::State::default()));
    let host = use_hook({
        let window = window.clone();
        move || {
            native::Host(Rc::new(RefCell::new(native::HostState {
                root,
                main: window,
                windows: HashMap::new(),
                tray: None,
                tray_menu: None,
            })))
        }
    });
    native::window_events(client.clone(), 1);
    use_future({
        let client = client.clone();
        let window = window.clone();
        move || {
            let client = client.clone();
            let window = window.clone();
            async move {
                loop {
                    client.notify("displays", json!({"displays":native::displays(&window),"cursor":native::cursor(&window)}));
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    });
    use_future({
        let client = client.clone();
        let host = host.clone();
        move || {
            let client = client.clone();
            let host = host.clone();
            async move {
                if std::env::args().any(|a| a == "--smoke-test") {
                    if let Some(path) = std::env::args().find_map(|a| a.strip_prefix("--preview-notice=").map(str::to_owned)) {
                        let result = match smoke::preview_notice(host.clone(), state, &path).await {
                            Ok(value) => value,
                            Err(error) => json!({"passed":false,"error":error}),
                        };
                        if let Some(report) = std::env::args().find_map(|a| a.strip_prefix("--preview-report=").map(str::to_owned)) {
                            let _ = std::fs::write(report, serde_json::to_string_pretty(&result).unwrap());
                        }
                        return;
                    }
                }
                if let Some(directory) = std::env::args()
                    .find_map(|arg| arg.strip_prefix("--ui-audit=").map(str::to_owned))
                {
                    let result =
                        match smoke::audit(client.clone(), host.clone(), state, &directory).await {
                            Ok(value) => value,
                            Err(error) => json!({"passed":false,"error":error}),
                        };
                    let _ = std::fs::write(
                        std::path::Path::new(&directory).join("real-ui-audit.json"),
                        serde_json::to_string_pretty(&result).unwrap(),
                    );
                }
                if let Some(report) = std::env::args()
                    .find_map(|arg| arg.strip_prefix("--smoke-report=").map(str::to_owned))
                {
                    if !std::env::args().any(|arg| arg == "--smoke-test") {
                        return;
                    }
                    let result = match smoke::run(client.clone(), host, state).await {
                        Ok(result) => result,
                        Err(error) => json!({"passed":false,"error":error}),
                    };
                    let _ = std::fs::write(report, serde_json::to_string_pretty(&result).unwrap());
                    let _ = client
                        .invoke(1, "application:confirm-close", json!(["keep"]))
                        .await;
                }
            }
        }
    });
    // Dioxus 0.7 shares muda between menubar and tray. Its first global
    // handler routes tray menu choices as MudaMenuEvent. Accept both routes.
    desktop::use_muda_event_handler({
        let client = client.clone();
        move |event| {
            if let Some(index) = event
                .id
                .0
                .strip_prefix("nogirem-")
                .and_then(|s| s.parse::<u64>().ok())
            {
                client.notify("tray", json!({"event":"menu","index":index}));
            }
        }
    });
    desktop::use_tray_menu_event_handler({
        let client = client.clone();
        move |event| {
            if let Some(index) = event
                .id
                .0
                .strip_prefix("nogirem-")
                .and_then(|s| s.parse::<u64>().ok())
            {
                client.notify("tray", json!({"event":"menu","index":index}));
            }
        }
    });
    desktop::use_tray_icon_event_handler({
        let client = client.clone();
        move |event| {
            let name = match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => "click",
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => "double-click",
                _ => return,
            };
            client.notify("tray", json!({"event":name}));
        }
    });
    use_future(move || {
        let client = client.clone();
        let host = host.clone();
        let mut incoming = incoming.borrow_mut().take().expect("one desktop receiver");
        async move {
            while let Some(message) = incoming.recv().await {
                if message["type"] == "disconnect" {
                    state.write().error =
                        "최적화 서비스 연결이 종료되었습니다. 앱을 다시 실행해 주세요.".into();
                    break;
                }
                let request = message["type"] == "request";
                let (method, params) = if request {
                    (message["method"].as_str().unwrap_or(""), &message["params"])
                } else {
                    (
                        message["params"]["method"].as_str().unwrap_or(""),
                        &message["params"]["params"],
                    )
                };
                if method == "window.event" && params["windowId"] == 1 {
                    ui::receive(
                        state,
                        params["channel"].as_str().unwrap_or(""),
                        &params["args"][0],
                    );
                }
                let result = host.dispatch(&client, method, params).await;
                if method == "window.open" && params["windowId"] == 1 && result.is_ok() {
                    state.write().ready = true;
                    let client = client.clone();
                    spawn(async move { ui::initialize(client, state).await });
                }
                if request {
                    let reply = match result {
                        Ok(result) => json!({"type":"response","id":message["id"],"result":result}),
                        Err(error) => json!({"type":"response","id":message["id"],"error":error}),
                    };
                    let _ = client.send(reply);
                } else if let Err(error) = result {
                    state.write().error = error;
                }
            }
        }
    });
    rsx! { ui::Main {} }
}
