use crate::bridge::Client;
use dioxus::{desktop, prelude::*};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq)]
pub struct ColorTransition { pub x: f64, pub y: f64, pub paused: bool }

#[derive(Default, Clone)]
pub struct State {
    pub ready: bool,
    pub startup_data_ready: bool,
    pub interface_visible: bool,
    pub identity: String,
    pub controls_entered: bool,
    pub navigation_ready: bool,
    pub creator_phase: String,
    pub tab_phase: String,
    pub notices: std::collections::BTreeMap<String, String>,
    pub error: String,
    pub services: Value,
    pub blackbox: Value,
    pub turbo: Value,
    pub input: Value,
    pub startup_tray: bool,
    pub startup_tray_supported: bool,
    pub muted: bool,
    pub busy: bool,
    pub boost_action: Option<bool>,
    pub color_transition: Option<ColorTransition>,
    pub modal: String,
    pub tab: String,
    pub notice: Value,
    pub reports: Value,
    pub update: Value,
    pub update_dismissed: bool,
    pub seen_report_ids: std::collections::BTreeSet<String>,
    pub include_nic: bool,
    pub creator_profile: Value,
    pub visual_active: bool,
    pub creator_prompt: bool,
    pub creator_prompt_loaded: bool,
    pub document_hidden: bool,
    pub pending: std::collections::BTreeSet<&'static str>,
}

pub fn receive(mut state: Signal<State>, channel: &str, value: &Value) {
    let mut state = state.write();
    apply_event(&mut state, channel, value);
}

fn apply_event(state: &mut State, channel: &str, value: &Value) {
    match channel {
        "application:update-state-changed" => state.update=value.clone(),
        "optimization:graphics-status-changed" => state.services["graphics"] = value.clone(),
        "optimization:dxvk-status-changed" => state.services["affinity"]["dxvk"] = value.clone(),
        "application:blackbox-status-changed" => {
            if !state.blackbox.is_object() {
                state.blackbox = json!({});
            }
            if let Some(fields) = value.as_object() {
                for (key, value) in fields {
                    state.blackbox[key] = value.clone();
                }
            }
        }
        "application:close-requested" => state.modal = "close".into(),
        "application:visual-activity-changed" => {
            state.visual_active = value.as_bool().unwrap_or(false)
        }
        "application:notice-available" => {
            if value["id"].as_str().is_none() || value["markdown"].as_str().is_none() { return; }
            state.notice = value.clone();
            if state.modal.is_empty() {
                state.modal = "notice".into();
            }
        }
        "application:report-responses-available" => {
            let Some(incoming) = value.as_array() else { return; };
            let mut queue = state.reports.as_array().cloned().unwrap_or_default();
            for reply in incoming {
                let Some(id) = reply["responseId"].as_str().filter(|s| !s.is_empty()) else { continue; };
                if state.seen_report_ids.insert(id.to_owned()) { queue.push(reply.clone()); }
            }
            state.reports = json!(queue);
            if !queue.is_empty()
                && (state.modal.is_empty() || state.modal == "notice")
            {
                state.modal = "report".into();
            }
        }
        _ => {}
    }
}

fn error_text(value: &Value) -> Option<&str> {
    value.as_str().or_else(|| value["message"].as_str())
}

fn unwrap_status(value: Value) -> Value {
    let mut result = json!({});
    if let Some(services) = value.as_object() {
        for (key, service) in services {
            result[key] = if service["ok"] == false {
                json!({"error":error_text(&service["error"]).unwrap_or("상태를 확인하지 못했습니다")})
            } else if service.get("ok").is_some() {
                service["data"].clone()
            } else {
                service.clone()
            };
        }
    }
    result
}

pub async fn initialize(client: Client, mut state: Signal<State>) {
    match client
        .invoke(1, "application:get-launch-context", json!([]))
        .await
    {
        Ok(context) => {
            let mut state = state.write();
            state.services = unwrap_status(context["optimizationStatus"].clone());
            state.startup_data_ready = true;
            state.blackbox = context["blackboxSetting"].clone();
            state.muted = context["startupMusicMuted"].as_bool().unwrap_or(false);
            state.include_nic = state.services["affinity"]["includeNic"]
                .as_bool()
                .unwrap_or(true);
        }
        Err(error) => state.write().error = error,
    }
    for (channel, key) in [
        ("application:get-turbo-key-setting", "turbo"),
        ("application:get-input-guard-setting", "input"),
        ("application:get-startup-tray-setting", "startup"),
        ("application:get-creator-channel", "creator"),
        ("application:get-creator-prompt-dismissed", "creator-prompt"),
        ("application:get-update-state", "update"),
        ("application:get-notice", "notice"),
        ("application:get-report-responses", "report"),
        ("application:get-visual-activity", "visual"),
    ] {
        let client = client.clone();
        spawn(async move {
            match client.invoke(1, channel, json!([])).await {
                Ok(value) => match key {
                    "turbo" => state.write().turbo = value,
                    "input" => state.write().input = value,
                    "creator" => state.write().creator_profile = value,
                    "creator-prompt" => {let mut s=state.write();s.creator_prompt=value!=true;s.creator_prompt_loaded=true;},
                    "update" => state.write().update=value,
                    "notice" => {
                        if !value.is_null() {
                            receive(state, "application:notice-available", &value);
                        }
                    }
                    "report" => receive(state, "application:report-responses-available", &value),
                    "visual" => state.write().visual_active = value.as_bool().unwrap_or(false),
                    _ => {
                        let mut s = state.write();
                        s.startup_tray = value["enabled"].as_bool().unwrap_or(false);
                        s.startup_tray_supported = value["supported"] == true;
                    }
                },
                Err(error) => {
                    if key=="creator-prompt" {let mut s=state.write();s.creator_prompt=true;s.creator_prompt_loaded=true;}
                    else {state.write().error=error;}
                },
            }
        });
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        for (channel, key) in [
            ("optimization:get-affinity-runtime", "affinity"),
            ("optimization:get-memory-runtime", "memory"),
        ] {
            match client.invoke(1, channel, json!([])).await {
                Ok(value) => {
                    let mut state = state.write();
                    if !state.services.is_object() {
                        state.services = json!({});
                    }
                    if let Some(fields) = value.as_object() {
                        if !state.services[key].is_object() {
                            state.services[key] = json!({});
                        }
                        for (field, value) in fields {
                            state.services[key][field] = value.clone();
                        }
                    }
                }
                Err(error) => {
                    state.write().error = error;
                    return;
                }
            }
        }
    }
}

fn command(client: Client, mut state: Signal<State>, channel: &'static str, args: Value) {
    spawn(async move {
        // Event arguments may contain a temporary signal read. Mutate only after
        // the handler returns, so that read has been released.
        if !state.write().pending.insert(channel) {
            return;
        }
        state.write().busy = true;
        let boost_command = matches!(channel,"optimization:set-frame-boost-enabled" | "optimization:reset-frame-boost");
        if boost_command { state.write().boost_action = Some(args[0]["enabled"].as_bool().unwrap_or(false)); }
        match client.invoke(1, channel, args).await {
            Ok(value) => {
                let mut state = state.write();
                if channel == "application:export-diagnostic-logs" {
                    state.notices.insert(
                        channel.into(),
                        if value["canceled"] == true {
                            "로그 추출을 취소했습니다".into()
                        } else {
                            "로그 파일을 저장했습니다".into()
                        },
                    );
                }
                match channel {
                    "application:check-update" | "application:install-update" => { if value["phase"].is_string() { state.update=value; } },
                    "optimization:get-status" => state.services = unwrap_status(value),
                    "optimization:set-frame-boost-enabled" | "optimization:reset-frame-boost" => {
                        state.services["affinity"] = value["affinity"].clone();
                        state.services["memory"] = value["memory"].clone();
                        let paused = !(state.services["affinity"]["running"] == true && state.services["memory"]["running"] == true);
                        if state.color_transition.is_some_and(|t| t.paused != paused) { state.color_transition = None; }
                    }
                    "optimization:optimize-graphics" | "optimization:refresh-graphics" => {
                        state.services["graphics"] = value
                    }
                    "optimization:optimize-network"
                    | "optimization:restore-network"
                    | "optimization:refresh-network" => state.services["network"] = value,
                    "application:set-turbo-key-setting"
                    | "application:download-turbo-key-helper"
                    | "application:remove-turbo-key-helper" => state.turbo = value,
                    "application:set-input-guard-setting" => state.input = value,
                    "application:set-blackbox-feature-enabled"
                    | "application:set-blackbox-enabled" => state.blackbox = value,
                    "application:set-startup-tray-setting" => {
                        state.startup_tray = value["enabled"].as_bool().unwrap_or(false)
                    }
                    "application:set-startup-music-setting" => {
                        state.muted = value["muted"].as_bool().unwrap_or(false)
                    }
                    "application:confirm-close" => state.modal = next_alert(&state),
                    "application:dismiss-creator-prompt" => state.creator_prompt = false,
                    "application:dismiss-notice" => {
                        state.notice = Value::Null;
                        state.modal = next_alert(&state);
                    }
                    "application:acknowledge-report-response" => {
                        if let Some(reports) = state.reports.as_array_mut() {
                            if !reports.is_empty() {
                                reports.remove(0);
                            }
                        }
                        if state.reports.as_array().is_none_or(Vec::is_empty) {
                            state.modal = if state.notice.is_null() {
                                String::new()
                            } else {
                                "notice".into()
                            };
                        }
                    }
                    "optimization:set-game-cpu-core-count"
                    | "optimization:refresh-game-cpu-core-setting"
                    | "optimization:run-cpu-reorder" => state.services["affinity"] = value,
                    _ => {}
                }
            }
            Err(error) => { let mut s=state.write();s.error=error;if boost_command {s.color_transition=None;} },
        }
        let mut state = state.write();
        if boost_command { state.boost_action=None; }
        state.pending.remove(channel);
        state.busy = !state.pending.is_empty();
    });
}

fn next_alert(state: &State) -> String {
    if state
        .reports
        .as_array()
        .is_some_and(|reports| !reports.is_empty())
    {
        "report".into()
    } else if !state.notice.is_null() {
        "notice".into()
    } else {
        String::new()
    }
}

fn close_modal(client: Client, state: Signal<State>) {
    if state.peek().busy {
        return;
    }
    let closing_modal = state.peek().modal.clone();
    let closing_tab = state.peek().tab.clone();
    spawn(async move {
        let _ = document::eval("document.querySelectorAll('.modal-backdrop,.modal-sheet,.terms-backdrop,.terms-modal,.notice-backdrop,.notice-dialog').forEach(e=>e.classList.add('closing'));").await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if state.peek().modal == closing_modal && state.peek().tab == closing_tab {
            close_modal_now(client, state);
        }
    });
}

fn close_modal_now(client: Client, mut state: Signal<State>) {
    if state.peek().busy {
        return;
    }
    let snapshot = state.peek().clone();
    match snapshot.modal.as_str() {
        "close" => command(
            client,
            state,
            "application:confirm-close",
            json!(["cancel"]),
        ),
        "notice" => command(
            client,
            state,
            "application:dismiss-notice",
            json!([snapshot.notice["id"]]),
        ),
        "report" => command(
            client,
            state,
            "application:acknowledge-report-response",
            json!([snapshot.reports[0]["responseId"]]),
        ),
        "" => state.write().tab.clear(),
        _ => state.write().modal = next_alert(&snapshot),
    }
}

#[component]
fn Action(
    label: String,
    channel: &'static str,
    #[props(default = json!([]))] args: Value,
    #[props(default = "setting-action".into())] class: String,
    #[props(default = false)] disabled: bool,
    #[props(default)] children: Element,
    #[props(default = false)] label_span: bool,
) -> Element {
    let client = use_context::<Client>();
    let state = use_context::<Signal<State>>();
    let lifecycle = matches!(
        channel,
        "application:request-close" | "application:minimize-to-tray" | "application:confirm-close"
    );
    rsx! { button { class, disabled: disabled || !state.read().ready || state.read().pending.contains(channel) || (state.read().busy && !lifecycle),
    onclick: move |_| command(client.clone(), state, channel, args.clone()), {children} if !label.is_empty() { if label_span { span { "{label}" } } else { "{label}" } } } }
}

#[component]
pub fn Main() -> Element {
    let mut state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let window = desktop::use_window();
    use_future({
        let client = client.clone();
        move || {
            let client = client.clone();
            async move {
                let mut eval = document::eval(
                    r#"
                    const {createSmoothWheelScroller} = await import('/web/smooth-wheel-scroll.mjs');
                    const scroller = createSmoothWheelScroller(()=>document.querySelector('.creator-content'));
                    window.nogiremCreatorScroller = scroller;
                    document.addEventListener('wheel', e => { if(e.target.closest?.('.creator-content')) scroller.handleWheel(e); }, {passive:false});
                    const visibility=()=>dioxus.send({documentHidden:document.hidden});
                    document.addEventListener('visibilitychange',visibility);visibility();
                    document.addEventListener('keydown', e => {
                        if (e.key === 'Escape' && !e.repeat && !e.defaultPrevented) { e.preventDefault(); dioxus.send({escape:true}); }
                        if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r')) e.preventDefault();
                    });
                    document.addEventListener('click', e => {
                        const link = e.target.closest?.('a.markdown-link[href]');
                        if (link) { e.preventDefault(); if (/^https?:/.test(link.href)) dioxus.send({url:link.href}); }
                    });
                    await new Promise(() => {});
                "#,
                );
                while let Ok(message) = eval.recv::<Value>().await {
                    if let Some(hidden)=message["documentHidden"].as_bool() {state.write().document_hidden=hidden;}
                    else if message["escape"] == true {
                        close_modal(client.clone(), state);
                    } else if let Some(url) = message["url"].as_str() {
                        command(
                            client.clone(),
                            state,
                            "application:open-notice-link",
                            json!([url]),
                        );
                    }
                }
            }
        }
    });
    let snapshot = state.read().clone();
    let running = snapshot.services["affinity"]["running"] == true
        && snapshot.services["memory"]["running"] == true;
    let any_running = snapshot.services["affinity"]["running"] == true
        || snapshot.services["memory"]["running"] == true;
    let active = snapshot.services["affinity"]["gameActive"] == true
        || snapshot.services["memory"]["gameActive"] == true;
    let status = if !snapshot.ready {
        "초기화 중"
    } else if snapshot.boost_action == Some(false) {
        "부스트 중단중"
    } else if !snapshot.boost_action.unwrap_or(running) {
        "부스트 적용 중단됨"
    } else if active {
        "실시간 부스트중"
    } else {
        "부스트 대기중"
    };
    let creator = !snapshot.tab.is_empty();
    let visual_running = snapshot.color_transition.map(|t|t.paused)
        .unwrap_or_else(||snapshot.boost_action.unwrap_or(running));
    rsx! {
        document::Stylesheet { href: "/web/styles.css" }
        document::Stylesheet { href: "/web/update-preview.css" }
        style { {include_str!("ui.css")} }
        style { {include_str!("modal.css")} }
        style { {include_str!("terms.css")} }
        style { {include_str!("notice.css")} }
        crate::wave::Wave {}
        div { class: if creator { "window-drag creator-active" } else { "window-drag" }, onmousedown: {let window=window.clone(); move |_| window.drag()} }
        if creator { div { class: "window-drag creator-window-drag-right", onmousedown: {let window=window.clone(); move |_| window.drag()} } }
        div { class: "window-controls",
            button { class: "window-control window-minimize", aria_label: "트레이로 최소화", onclick: {let client=client.clone(); move |_| command(client.clone(), state, "application:minimize-to-tray", json!([])) }, svg { width: "25", height: "25", "aria-hidden": "true", class: "control-icon-bg",line { x1: "0", y1: "24", x2: "25", y2: "24", stroke_width: "2", class: "" }}svg { width: "25", height: "25", "aria-hidden": "true", line { x1: "0", y1: "24", x2: "25", y2: "24", stroke_width: "2", class: "control-line-one" }} }
            button { class: "window-control window-close", aria_label: "프로그램 닫기", onclick: {let client=client.clone(); move |_| command(client.clone(), state, "application:request-close", json!([])) }, svg { width: "25", height: "25", "aria-hidden": "true", class: "control-icon-bg",line { x1: "0", y1: "0", x2: "25", y2: "25", stroke_width: "2", class: "" }line { x1: "25", y1: "0", x2: "0", y2: "25", stroke_width: "2", class: "" }}svg { width: "25", height: "25", "aria-hidden": "true", line { x1: "0", y1: "0", x2: "25", y2: "25", stroke_width: "2", class: "control-line-one" }line { x1: "25", y1: "0", x2: "0", y2: "25", stroke_width: "2", class: "control-line-two" }} }
        }
        button { class: format!("app-version{}{}", if visual_running { "" } else { " paused" }, if update_available(&snapshot.update) { " update-available" } else { "" }), aria_label: if update_available(&snapshot.update) { "새 버전 업데이트" } else { "최신 업데이트 확인" }, onclick: {let client=client.clone();move |_| {state.write().update_dismissed=false;command(client.clone(),state,"application:check-update",json!([]))}}, if update_available(&snapshot.update) { "새 버전 출시됨" } else { {env!("CARGO_PKG_VERSION")} } }

        CreatorPrompt { visible: creator_prompt_visible(&snapshot) }
        button { class: format!("creator-credit {} {} {}",if snapshot.navigation_ready {"entered"}else{""},if visual_running {""}else{"paused"},if !snapshot.modal.is_empty() {"hidden"}else{""}),
            aria_label: if creator {"기존 화면으로 돌아가기"} else {"제작자 소개 열기"},
            disabled: !snapshot.interface_visible || snapshot.identity!="done" || !snapshot.navigation_ready || !snapshot.modal.is_empty() || matches!(snapshot.creator_phase.as_str(),"opening"|"closing") || snapshot.color_transition.is_some(),
            onclick: {let client=client.clone();move |_| {
                if state.peek().tab.is_empty() {
                    state.write().creator_prompt=false;
                    let client=client.clone();spawn(async move {let _=client.invoke(1,"application:dismiss-creator-prompt",json!([])).await;});
                }
                creator_toggle(state);
            }}, "[류트@렘] 제작"
        }
        main { class: if snapshot.interface_visible { "compact-shell interface-ready" } else { "compact-shell interface-starting" },
            section { class: format!("boost-home {} {} {}", if creator { format!("creator-{}", if snapshot.creator_phase.is_empty() { "open" } else { &snapshot.creator_phase }) } else { String::new() }, if visual_running { "" } else { "paused" }, if status == "부스트 대기중" && snapshot.identity == "done" { "waiting" } else { "" }),
                img { src: "/public/logo3.png", alt: "", draggable: "false" }
                button { style: if snapshot.identity == "done" { "display:grid" } else { "display:none" }, class: if snapshot.controls_entered { "optimization-summary entered" } else { "optimization-summary" }, onclick: move |_| state.write().modal = "optimization".into(),
                    div { class: if snapshot.services["graphics"]["allMet"] == true { "ready" } else { "" }, StatusIcon { ready: snapshot.services["graphics"]["allMet"] == true } span { "그래픽 설정 " if snapshot.services["graphics"]["supported"] == true && snapshot.services["graphics"]["detected"] == true && snapshot.services["graphics"]["allMet"] == true { "최적화됨" } else { "확인 필요" } } }
                    div { class: if snapshot.services["network"]["optimized"] == true { "ready" } else { "" }, StatusIcon { ready: snapshot.services["network"]["optimized"] == true } span { "네트워크 " if snapshot.services["network"]["optimized"] == true { "최적화됨" } else { "확인 필요" } } }
                }
                Action { label_span: true, label: "주변 캐릭터 강제 간소화", channel: "application:open-character-guide", class: format!("character-guide-link {} {}", if snapshot.controls_entered { "entered" } else { "" }, if snapshot.services["affinity"]["characterSimplification"]["applied"] == true { "ready" } else { "warning" }), StatusIcon { ready: snapshot.services["affinity"]["characterSimplification"]["applied"] == true } }
                Action { label_span: true, label: dxvk_label(&snapshot.services["affinity"]), channel: "application:open-dxvk-manager", class: format!("dxvk-update-link {} {}", if snapshot.controls_entered { "entered" } else { "" }, dxvk_class(&snapshot.services["affinity"])), StatusIcon { ready: dxvk_class(&snapshot.services["affinity"]) == "ready", checking: dxvk_class(&snapshot.services["affinity"]) == "checking" } }
                if snapshot.blackbox["featureEnabled"] == true { BlackboxControls {} }
                button { class: if status.contains("중단") { "boost-text-area paused-target" } else { "boost-text-area" }, disabled: !snapshot.ready || snapshot.busy || snapshot.identity != "done" || snapshot.color_transition.is_some() || (running && !active),
                    onclick: move |event| {
                        if running && !active { return; }
                        let p = event.client_coordinates();
                        state.write().color_transition=Some(ColorTransition{x:p.x,y:p.y,paused:any_running});
                        let _ = document::eval(&format!("window.nogiremWave?.makeActionWave({}, {}, {})", p.x, p.y, any_running));
                        command(client.clone(), state, "optimization:set-frame-boost-enabled", json!([{ "enabled":!any_running, "includeNic":state.peek().include_nic }]));
                    },
                    match snapshot.identity.as_str() {
                        "brand" => rsx! { span { class: "startup-identity-text", "마비노기 렘 부스터" } },
                        "transition" => rsx! { span { class: "startup-identity-text identity-roulette-out", "마비노기 렘 부스터" } span { class: "startup-identity-text identity-roulette-in", onanimationend: move |_| crate::wave::finish_identity(state), "공개 사용자 버전" } },
                        "version" => rsx! { span { class: "startup-identity-text", "공개 사용자 버전" } },
                        "done" => rsx! { StatusText { text: status.to_owned() } },
                        _ => rsx! {},
                    }
                }
                if snapshot.identity == "done" {
                    span { class: if status == "부스트 대기중" { "status-guide waiting" } else { "status-guide" }, span { key: "{status}", class: "guide-text guide-text-in", if status == "실시간 부스트중" { "마비노기를 위해 모든 프로세스를 최적화 하고 있습니다" } else if status == "부스트 대기중" { "마비노기 클라이언트를 기다리고 있습니다" } else if status == "부스트 중단중" { "적용한 부스트 설정을 원상복구하고 있습니다" } else { "부스트 적용이 중단되어 모든 설정을 원상복구했습니다" } } }
                }
            }
            if creator { Creator {} }
            if let Some(ColorTransition{x,y,paused}) = snapshot.color_transition {
                section { class: if paused { "boost-home boost-color-overlay paused" } else { "boost-home boost-color-overlay" },
                    style: format!("--wave-x:{x}px;--wave-y:{y}px;--wave-radius:{}px;--wave-duration:{}ms;--wave-delay:{}ms", if paused {420} else {600}, if paused {600} else {3000}, if paused {0} else {500}),
                    "aria-hidden": "true", onanimationend: move |event: Event<AnimationData>| {
                        if event.animation_name() == "boost-color-reveal" { state.write().color_transition=None; }
                    },
                    img { src: "/public/logo3.png", alt: "", draggable: "false" }
                }
            }
        }
        if snapshot.modal != "close" && !snapshot.update_dismissed && matches!(snapshot.update["phase"].as_str(),Some("downloading"|"downloaded"|"installing"|"error")) { UpdatePreview {} }
        if !snapshot.modal.is_empty() { Modal {} }
        if !snapshot.error.is_empty() {
            div { class: "native-error", role: "alert",
                p { "{snapshot.error}" }
                button { onclick: move |_| state.write().error.clear(), "확인" }
            }
        }
    }
}

#[component]
fn Creator() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let snapshot = state.read().clone();
    rsx! {
        section { class: format!("creator-view {} {}", if snapshot.creator_phase.is_empty() { "open" } else { &snapshot.creator_phase }, if snapshot.services["affinity"]["running"] == true && snapshot.services["memory"]["running"] == true { "" } else { "paused" }), aria_label: "제작자 및 프로그램 정보",
            button { class: "creator-return", disabled: snapshot.creator_phase != "open", onclick: move |_| creator_toggle(state), svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: "m5.7 8.3 6.3 6.3 6.3-6.3 1.4 1.4-7.7 7.7-7.7-7.7 1.4-1.4Z" } } }
            nav { class: "creator-tabs",
                for (id, label) in [("developer", "소개"), ("operation", "작동 원리"), ("donation", "후원 / 기부"), ("history", "버전 변경 기록"), ("developer-tools", "고급 기능")] {
                    button { class: if snapshot.tab == id { "active" } else { "" }, disabled: snapshot.creator_phase != "open" || !snapshot.tab_phase.is_empty(), onclick: move |_| creator_tab(state, id), "{label}" }
                }
            }
            section { class: "creator-content", div { class: format!("creator-content-inner {}", snapshot.tab_phase),
                match snapshot.tab.as_str() {
                    "developer-tools" => rsx! { Advanced {} },
                    "donation" => rsx! { div { class: "donation-grid",
                        article { class: "donation-card", header { img { src: snapshot.creator_profile["avatarDataUrl"].as_str().unwrap_or("/web/creator-channel-avatar.jpg"), alt: "" } div { small { "개인 후원" } h2 { "류트 서버 렘" } } } p { "기능에 대한 문의나 간단한 커피값 지원 등은 마비노기 류트 서버의 렘 캐릭터로 연락해 주세요." } }
                        Action { label: "", channel: "application:open-direct-donation", class: "donation-card direct-donation-card", header { img { src: "/web/direct-donation-logo.svg", alt: "" } div { small { "추천 기부처" } h2 { "곧장기부" } } } p { "SK그룹이 지원하는 행복나눔재단에서 운영하며, 기부금이 수수료 없이 100% 전달되는 것이 특징입니다." } }
                    } },
                    "operation" => rsx! { div { class: "introduce-markdown operation-markdown", dangerous_inner_html: crate::markdown::blocks(include_str!("../../OPERATION.md"),false,false) } },
                    "history" => rsx! { VersionHistory {} },
                    _ => rsx! { div { class: "creator-introduction",
                        button { class: "creator-channel-card", onclick: move |_| command(client.clone(), state, "application:open-creator-channel", json!([])),
                            img { src: snapshot.creator_profile["avatarDataUrl"].as_str().unwrap_or("/web/creator-channel-avatar.jpg"), alt: "", draggable: "false" }
                            span { class: "creator-channel-copy",
                                strong { {snapshot.creator_profile["name"].as_str().unwrap_or("마비노기 렘")} }
                                if !snapshot.creator_profile.is_null() { span { {snapshot.creator_profile["handle"].as_str().unwrap_or("")} } small { {creator_statistics(&snapshot.creator_profile)} } }
                            }
                            svg { class: "youtube-logo", view_box: "0 0 24 24", "aria-hidden": "true", path { d: "M23.5 6.2a3 3 0 0 0-2.1-2.1C19.5 3.6 12 3.6 12 3.6s-7.5 0-9.4.5A3 3 0 0 0 .5 6.2 31 31 0 0 0 0 12a31 31 0 0 0 .5 5.8 3 3 0 0 0 2.1 2.1c1.9.5 9.4.5 9.4.5s7.5 0 9.4-.5a3 3 0 0 0 2.1-2.1A31 31 0 0 0 24 12a31 31 0 0 0-.5-5.8ZM9.6 15.6V8.4l6.3 3.6-6.3 3.6Z" } }
                        }
                        Markdown { text: include_str!("../../INTRODUCE.md") }
                    } },
                }
            } }
        }
    }
}

#[component]
fn Advanced() -> Element {
    let mut state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let s = state.read().clone();
    let input_loaded = s.input.is_object();
    let turbo_loaded = s.turbo.is_object();
    rsx! { div { class: "developer-tools",
        div { class: "developer-tool-row",
            div { h2 { "버그 리포트" } p { "문제가 발생한 경우 로그 추출 파일을 전송해 주세요" } }
            div { class: "developer-tool-actions",
                Action { label: "로그 추출", channel: "application:export-diagnostic-logs" }
                Action { label: "제출하기", channel: "application:open-bug-report-form", class: "developer-tool-secondary" }
            }
        }
        if let Some(notice) = s.notices.get("application:export-diagnostic-logs") { span { class: "developer-tool-status", "{notice}" } }
        div { class: "developer-tool-row",
            div { h2 { "Windows 시작 시 트레이 실행" } p { "로그인하면 창과 시작 음악 없이 백그라운드에서 실행합니다" } }
            Action { label: if s.startup_tray { "사용 중" } else { "사용하기" }, channel: "application:set-startup-tray-setting", disabled: !s.startup_tray_supported, args: json!([!s.startup_tray]), class: if s.startup_tray { "active" } else { "" } }
        }
        div { class: "developer-tool-row",
            div { h2 { "시작 음악 음소거" } p { "프로그램을 실행할 때 나오는 시작 음악을 재생하지 않습니다" } }
            Action { label: if s.muted { "사용 중" } else { "사용하기" }, channel: "application:set-startup-music-setting", args: json!([!s.muted]), class: if s.muted { "active" } else { "" } }
        }
        div { class: "developer-tool-stack", CpuSettings {} }
        div { class: "developer-tool-row",
            div { h2 { "터보 키" } p { if s.turbo["installed"] == true { "키를 누르고 있으면 해당 키를 반복해서 연타합니다" } else { "스킬 키를 미리 누르고 있어도 스킬이 사용될 수 있도록 합니다. 기능을 사용하려면 다운로드가 필요합니다" } } }
            div { class: "developer-tool-actions",
                if s.turbo["installed"] == true {
                    div { class: "turbo-key-installed-actions",
                        div { class: "turbo-key-primary-actions",
                            if s.turbo["enabled"] == true {
                                button { class: "developer-tool-secondary", disabled: !turbo_loaded || s.busy, onclick: move |_| state.write().modal = "turbo".into(), "키 설정" }
                                Action { label: if s.turbo["running"] == true { "사용 중" } else { "실행 오류" }, channel: "application:set-turbo-key-setting", args: setting_patch(&s.turbo, "enabled", json!(false)), class: if s.turbo["running"] == true { "active" } else { "" } }
                            } else { button { disabled: !turbo_loaded || s.busy, onclick: move |_| state.write().modal = "turbo".into(), "사용하기" } }
                        }
                        Action { label: "터보키 제거하기", channel: "application:remove-turbo-key-helper", class: "turbo-key-remove" }
                    }
                } else { button { disabled: !turbo_loaded || s.busy, onclick: move |_| state.write().modal = "turbo".into(), "다운로드" } }
            }
        }
        if s.turbo["installed"] == true && s.turbo["enabled"] == true {
            label { class: "turbo-key-immediate-option",
                input { r#type: "checkbox", checked: s.turbo["ignoreInitialDelay"] == true, disabled: s.busy,
                    onchange: {let client=client.clone(); move |e: Event<FormData>| command(client.clone(), state, "application:set-turbo-key-setting", setting_patch(&state.peek().turbo, "ignoreInitialDelay", json!(e.checked()))) }
                } span { "키보드 입력 지연을 무시하고 즉시 입력" }
            }
        }
        div { class: "developer-tool-row developer-tool-row-nested overlay-tool-row",
            div { h2 { "오버레이" } p { "게임 화면의 원하는 영역을 복제해 화면 중앙 근처에 표시합니다" } }
            button { disabled: true, "개발 중" }
            div { class: "overlay-tool-example", span { "스킬 슬롯 지정" } svg { view_box: "0 0 20 12", "aria-hidden": "true", path { d: "M1 6h16m-4-4 4 4-4 4" } } strong { "쿨타임을 보기 쉬운 위치에 표시" } }
            small { class: "overlay-tool-download", "기능 모듈은 터보 키처럼 별도 다운로드 방식으로 제공될 예정입니다" }
        }
        div { class: "developer-tool-row",
            div { h2 { "게임 블랙박스" } p { "메인 화면에서 게임 화면 순환 녹화와 클립 저장 기능을 사용할 수 있습니다" } }
            Action { label: if s.blackbox["featureEnabled"] == true { "사용 중" } else { "사용하기" }, channel: "application:set-blackbox-feature-enabled", args: json!([s.blackbox["featureEnabled"] != true]), disabled: !s.blackbox.is_object(), class: if s.blackbox["featureEnabled"] == true { "active" } else { "" } }
        }
        div { class: "developer-tool-row",
            div { h2 { "Alt+Enter 방지" } p { "마비노기 플레이 중 전체 화면 전환 단축키 Alt+Enter 입력을 차단합니다" } }
            Action { label: if s.input["enabled"] == true { if s.input["running"] == true { "사용 중" } else { "실행 오류" } } else { "사용하기" }, channel: "application:set-input-guard-setting", args: setting_patch(&s.input, "enabled", json!(s.input["enabled"] != true)), disabled: !input_loaded, class: if s.input["enabled"] == true && s.input["running"] == true { "active" } else { "" } }
        }
        div { class: "developer-tool-row developer-tool-row-nested",
            div { h2 { "게임 마우스 커서 크기" } p { "마비노기 플레이 중에만 25%부터 800%까지 Windows 마우스 커서 크기를 변경합니다" } }
            select { class: "developer-tool-select", aria_label: "게임 마우스 커서 크기", value: s.input["cursorScalePercent"].to_string(), disabled: !input_loaded || s.busy,
                onchange: {let client=client.clone(); move |e: Event<FormData>| command(client.clone(), state, "application:set-input-guard-setting", setting_patch(&state.peek().input, "cursorScalePercent", json!(e.value().parse::<u32>().unwrap_or(100)))) },
                for percent in (25..=800).step_by(25) { option { value: "{percent}", "{percent}%" } }
            }
            label { class: "developer-tool-subsetting", span { "게임 중 휠로 25%씩 조절" }
                select { class: "developer-tool-select", aria_label: "커서 크기 휠 조절", value: s.input["cursorWheelModifier"].as_str().unwrap_or("disabled"), disabled: !input_loaded || s.busy,
                    onchange: {let client=client.clone(); move |e: Event<FormData>| command(client.clone(), state, "application:set-input-guard-setting", setting_patch(&state.peek().input, "cursorWheelModifier", json!(e.value()))) },
                    option { value: "disabled", "사용 안 함" } option { value: "control", "Ctrl + 휠" } option { value: "alt", "Alt + 휠" }
                }
            }
        }
        if let Some(error) = s.input["error"].as_str() { span { class: "developer-tool-status", "{error}" } }
        div { class: "developer-tool-row",
            div { h2 { "CPU 재정렬" } p { "CPU 격리 구성을 재설정하여 캐시 초기화를 유도합니다" } }
            Action { label: "CPU 재정렬", channel: "optimization:run-cpu-reorder", disabled: !(s.services["affinity"]["running"] == true && s.services["memory"]["running"] == true && s.services["affinity"]["gameActive"] == true) }
        }
    } }
}

fn setting_patch(setting: &Value, key: &str, value: Value) -> Value {
    let mut setting = setting.clone();
    if !setting.is_object() {
        setting = json!({});
    }
    setting[key] = value;
    json!([setting])
}

#[component]
fn Modal() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let snapshot = state.read().clone();
    let modal = use_memo(move || state.read().modal.clone());
    use_effect(move || {
        let _modal = modal.read();
        let _ = document::eval(
            "requestAnimationFrame(()=>document.querySelector('[role=dialog]')?.focus())",
        );
    });
    let network_channel: &'static str = if snapshot.modal == "network-restore" {
        "optimization:restore-network"
    } else {
        "optimization:optimize-network"
    };
    if snapshot.modal == "turbo" && snapshot.turbo["installed"] != true {
        return rsx! {TermsDialog {}};
    }
    if matches!(snapshot.modal.as_str(), "notice" | "report") {
        let alert_key = if snapshot.modal == "report" {
            format!("report:{}", snapshot.reports[0]["responseId"].as_str().unwrap_or(""))
        } else { format!("notice:{}", snapshot.notice["id"].as_str().unwrap_or("")) };
        return rsx! {NoticeDialog { key: "{alert_key}" }};
    }
    rsx! { div { class: if snapshot.modal == "turbo" && snapshot.turbo["installed"] == true { "modal-backdrop fullscreen" } else if snapshot.modal == "optimization" { "modal-backdrop large" } else { "modal-backdrop" },
        section { class: if snapshot.modal == "turbo" && snapshot.turbo["installed"] == true { "modal-sheet fullscreen" } else if snapshot.modal == "optimization" { "modal-sheet large" } else { "modal-sheet" }, role: "dialog", aria_modal: "true", tabindex: "-1",
            onkeydown: move |event| {
                if event.key() == Key::Tab {
                    event.prevent_default();
                    let backwards = event.modifiers().shift();
                    let _ = document::eval(&format!(r#"const nodes=[...document.querySelectorAll('[role=dialog] button:not(:disabled),[role=dialog] input:not(:disabled),[role=dialog] select:not(:disabled),[role=dialog] a[href]')].filter(e=>e.getClientRects().length); if(nodes.length){{const i=nodes.indexOf(document.activeElement);nodes[(i+{}+nodes.length)%nodes.length].focus();}}"#, if backwards { -1 } else { 1 }));
                }
            },
            img { class: "modal-watermark", src: "/public/logo2-white-transparent.png", alt: "", draggable: "false" }
            if snapshot.modal != "network-confirm" && snapshot.modal != "network-restore" {
                button { class: "modal-close native-modal-close", onclick: {let client=client.clone();move |_| close_modal(client.clone(), state)}, svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: "M5.3 4 12 10.7 18.7 4 20 5.3 13.3 12l6.7 6.7-1.3 1.3-6.7-6.7L5.3 20 4 18.7l6.7-6.7L4 5.3 5.3 4Z" } } }
            }
            match snapshot.modal.as_str() {
                "close" => rsx! {
                    p { class: "modal-eyebrow", "프로그램 종료" }
                    h2 { "프레임 부스트를 정지할까요?" }
                    div { class: "modal-content",
                        p { class: "modal-description", "부스트를 유지한 채로 종료하면 마비노기 외 프로그램들의 성능이 제한될 수 있습니다." }
                        div { class: "modal-actions",
                            Action { class: "secondary", label: "적용 유지 후 종료", channel: "application:confirm-close", args: json!(["keep"]) }
                            Action { class: "primary", label: "부스트 설정 되돌린 후 종료", channel: "application:confirm-close", args: json!(["reset"]) }
                        }
                    }
                },
                "optimization" => rsx! { OptimizationDetails {} },
                "network-confirm" | "network-restore" => rsx! {
                    p { class: "modal-eyebrow", if snapshot.modal == "network-restore" { "네트워크 설정 복원" } else { "네트워크 최적화" } }
                    h2 { if snapshot.modal == "network-restore" { "패스트핑 설정을 되돌립니다" } else { "네트워크 연결을 다시 시작합니다" } }
                    div { class: "modal-content",
                        p { class: "modal-description", if snapshot.modal == "network-restore" { "TCP ACK 빈도와 TCP No Delay를 이전 설정으로 되돌립니다. 기록이 없으면 두 옵션을 끄며 TCP 자동 조정은 유지합니다." } else { "TCP ACK 빈도 또는 TCP No Delay를 적용하려면 네트워크 어댑터를 다시 연결해야 합니다." } " 인터넷 연결이 잠시 끊길 수 있습니다. 계속하시겠습니까?" }
                        div { class: "modal-actions", button { class: "secondary", disabled: snapshot.busy, onclick: {let client=client.clone();move |_| close_modal(client.clone(),state)}, "취소" }
                            Action { class: "monochrome", label: if snapshot.modal == "network-restore" { "되돌리기" } else { "계속하기" }, channel: network_channel }
                        }
                    }
                },
                "graphics-confirm" => rsx! {
                    p { class: "modal-eyebrow", "AMD Radeon 최적화" } h2 { "Radeon 전역 설정을 변경합니다" }
                    div { class: "modal-content", p { class: "modal-description", "수직 동기화, Enhanced Sync, Anti-Lag, Chill 설정이 Radeon GPU 전역에 적용되며 다른 게임에서도 유지됩니다. 최대 프레임 제한은 변경하지 않습니다." }
                        div { class: "modal-actions", button { class: "secondary", disabled: snapshot.busy, onclick: {let client=client.clone();move |_| close_modal(client.clone(),state)}, "취소" } Action { class: "monochrome", label: "계속하기", channel: "optimization:optimize-graphics" } }
                    }
                },
                "reset" => rsx! { h2 { "부스트 설정 초기화" } p { class: "modal-description", "적용한 CPU 및 NIC 부스트 설정을 되돌립니다." } div { class: "modal-actions", button { class: "secondary", onclick: {let client=client.clone();move |_| close_modal(client.clone(),state)}, "취소" } Action { class: "monochrome", label: "초기화", channel: "optimization:reset-frame-boost" } } },
                "turbo" => rsx! { Turbo {} },
                "input" => rsx! { InputSettings {} },
                "cpu" => rsx! { CpuSettings {} },
                "notice" => rsx! { h2 { "공지" } Markdown { text: snapshot.notice["markdown"].as_str().unwrap_or("").to_owned() } Action { label: "확인", channel: "application:dismiss-notice", args: json!([snapshot.notice["id"]]) } },
                "report" => rsx! {
                    h2 { {snapshot.reports[0]["title"].as_str().unwrap_or("버그 리포트 답변").to_owned()} }
                    Markdown { text: snapshot.reports[0]["message"].as_str().unwrap_or("").to_owned() }
                    p { "리포트 고유값: " {snapshot.reports[0]["reportId"].as_str().unwrap_or("")} }
                    if let Some(version) = snapshot.reports[0]["minimumVersion"].as_str() { p { "반영 버전: {version}" } }
                    Action { label: "확인했습니다", channel: "application:acknowledge-report-response", args: json!([snapshot.reports[0]["responseId"]]) }
                },
                _ => rsx! { p { "{snapshot.modal}" } },
            }
        }
    } }
}

#[component]
fn OptimizationDetails() -> Element {
    let mut state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let s = state.read().clone();
    let graphics = &s.services["graphics"];
    let network = &s.services["network"];
    let fast = &network["fastPing"];
    let graphics_ready =
        graphics["supported"] == true && graphics["detected"] == true && graphics["allMet"] == true;
    let network_ready = network["optimized"] == true;
    let graphics_busy = s.pending.contains("optimization:optimize-graphics");
    let network_busy = s.pending.contains("optimization:optimize-network")
        || s.pending.contains("optimization:restore-network");
    rsx! {
        p { class: "modal-eyebrow", "시스템 최적화" }
        div { class: "modal-content",
            div { class: "optimization-modal-grid",
                section { class: "optimization-detail", aria_labelledby: "graphics-status-title",
                    header {
                        div { class: "detail-heading-copy", span { class: "detail-category", "그래픽 설정" } h3 { id: "graphics-status-title", {graphics["title"].as_str().unwrap_or("그래픽 설정")} } }
                        button { class: if graphics_ready { "detail-action complete" } else { "detail-action" }, disabled: s.busy || graphics["detected"] != true || graphics_ready,
                            onclick: { let client = client.clone(); move |_| {
                                if state.peek().services["graphics"]["vendor"] == "amd" { state.write().modal = "graphics-confirm".into(); }
                                else { command(client.clone(), state, "optimization:optimize-graphics", json!([])); }
                            } },
                            if graphics_busy { "적용 중" } else if graphics_ready { "완료됨" } else { "최적화" }
                        }
                    }
                    if let Some(error) = error_text(&graphics["error"]) { p { class: "detail-error", "{error}" } }
                    else if graphics["detected"] == true {
                        p { class: "detail-device", {graphics["gpus"].as_array().map(|gpus|gpus.iter().filter_map(|gpu|gpu["name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default()} }
                        dl { class: "detail-list",
                            if let Some(goals) = graphics["goalsList"].as_array() {
                                for goal in goals { div {
                                    dt { {goal["label"].as_str().unwrap_or("")} }
                                    dd { class: if goal["met"] == true { "ready" } else { "" },
                                        if goal["supported"] == false { "해당 없음" } else if !goal["error"].is_null() { "적용 실패" } else if goal["met"] == true { "완료" } else { "조정 필요" }
                                    }
                                } }
                            }
                        }
                        if let Some(goals) = graphics["goalsList"].as_array() { for goal in goals { if let Some(error) = error_text(&goal["error"]) { p { class: "detail-error", "{error}" } } } }
                    } else { p { class: "detail-empty", {graphics["reason"].as_str().unwrap_or("지원되는 GPU를 찾지 못했습니다")} } }
                }
                section { class: "optimization-detail", aria_labelledby: "network-status-title",
                    header {
                        div { class: "detail-heading-copy", span { class: "detail-category", "네트워크 상태" } h3 { id: "network-status-title", "패스트핑 적용" } }
                        button { class: if network_ready { "detail-action complete" } else { "detail-action" }, disabled: s.busy || network.is_null() || network_ready, onclick: move |_| state.write().modal = "network-confirm".into(),
                            if network_busy { "적용 중" } else if network_ready { "완료됨" } else { "최적화" }
                        }
                    }
                    if let Some(error) = error_text(&network["error"]) { p { class: "detail-error", "{error}" } }
                    else if network.is_object() {
                        div { class: "detail-device-row",
                            p { class: "detail-device", {if fast["supported"] == false { fast["reason"].as_str().unwrap_or("") } else { fast["current"]["interfaceAlias"].as_str().unwrap_or("기본 네트워크") }} }
                            button { class: "detail-restore", disabled: s.busy || network["originalStateRecorded"] != true, onclick: move |_| state.write().modal = "network-restore".into(), if s.pending.contains("optimization:restore-network") { "되돌리는 중" } else { "설정 되돌리기" } }
                        }
                        dl { class: "detail-list",
                            for (key, label) in [("TcpAckFrequency", "TCP ACK 빈도"), ("TCPNoDelay", "TCP No Delay")] {
                                div { dt { "{label}" } dd { class: if fast["supported"] == false || fast["current"][key] == 1 { "ready" } else { "" }, if fast["supported"] == false { "적용 생략" } else if fast["current"][key] == 1 { "완료" } else { "조정 필요" } } }
                            }
                            div { dt { "TCP 자동 조정" } dd { class: if network["tcpAutoTuning"]["optimized"] == true { "ready" } else { "" }, if network["tcpAutoTuning"]["optimized"] == true { "완료" } else { "조정 필요" } } }
                        }
                    } else { p { class: "detail-empty", "네트워크 상태를 확인하고 있습니다" } }
                }
            }
        }
    }
}

#[component]
fn Markdown(text: String) -> Element {
    rsx! { div { class: "introduce-markdown", dangerous_inner_html: crate::markdown::blocks(&text,false,false) } }
}

#[component]
fn Turbo() -> Element {
    let mut state = use_context::<Signal<State>>();
    let mut draft = use_signal(|| state.peek().turbo.clone());
    let mut accepted = use_signal(|| false);
    let client = use_context::<Client>();
    let layout =
        use_hook(|| serde_json::from_str::<Value>(include_str!("keyboard-layout.json")).unwrap());
    use_future(move || async move {
        let mut eval = document::eval(
            r#"const onKey=e=>{if(e.repeat||e.key==='Escape'||e.target.closest('select,input'))return;e.preventDefault();dioxus.send(e.keyCode)};window.addEventListener('keydown',onKey);window.__nogiremRemoveKeyboard=()=>window.removeEventListener('keydown',onKey);await new Promise(()=>{});"#,
        );
        while let Ok(code) = eval.recv::<u64>().await {
            let selectable: Vec<Vec<Value>> =
                serde_json::from_str(include_str!("keyboard.json")).unwrap();
            if selectable
                .iter()
                .flatten()
                .any(|k| k["code"] == code && k["disabled"] != true)
            {
                toggle_key(draft, code);
            }
        }
    });
    use_drop(|| {
        let _ = document::eval("window.__nogiremRemoveKeyboard?.()");
    });
    if state.read().turbo["installed"] != true {
        return rsx! { h2 { "터보 키 다운로드 전 확인" }
            Markdown { text: include_str!("../../TURBO_KEY_TERMS.md") }
            label { input { r#type: "checkbox", checked: accepted(), onchange: move |e| accepted.set(e.checked()) } "이용 안내를 읽고 동의합니다" }
            Action { label: "확인 후 다운로드", channel: "application:download-turbo-key-helper", disabled: !accepted() }
        };
    }
    rsx! {
        h2 { "터보 키 적용 대상 설정" }
        div { class: "turbo-key-picker",
            div { class: "turbo-key-picker-summary", p { "화면의 키를 클릭하거나 실제 키보드 키를 눌러 선택 또는 해제하세요" } }
            div { class: "turbo-keyboard", aria_label: "터보 키 선택용 키보드",
                div { class: "turbo-keyboard-function-row", for key in layout["functionKeyboardRow"].as_array().unwrap() { Keycap { spec: key.clone(), draft } } }
                div { class: "turbo-keyboard-body",
                    div { class: "turbo-keyboard-main", for row in layout["mainKeyboardRows"].as_array().unwrap() { div { class: "turbo-keyboard-row", for key in row.as_array().unwrap() { Keycap { spec: key.clone(), draft } } } } }
                    div { class: "turbo-keyboard-navigation", for key in layout["navigationKeys"].as_array().unwrap() { Keycap { spec: key.clone(), draft } } }
                    div { class: "turbo-keyboard-numpad", for key in layout["numpadKeys"].as_array().unwrap() { Keycap { spec: key.clone(), draft } } }
                }
            }
            div { class: "turbo-key-picker-actions",
                label { class: "turbo-key-interval", span { "입력 간격" }
                    select { value: draft.read()["intervalMs"].to_string(), onchange: move |e| draft.write()["intervalMs"] = json!(e.value().parse::<u32>().unwrap_or(1)),
                        for interval in [1,3,5,10,20,30] { option { value: "{interval}", "{interval} ms" } }
                    } span { class: "turbo-key-interval-hint", "저사양 PC에서는 입력 간격을 늘리세요" }
                }
                div { class: "turbo-key-picker-buttons",
                    button { class: "reset", onclick: move |_| {draft.write()["keys"]=json!([]);draft.write()["intervalMs"]=json!(1);}, "초기화" }
                    button { disabled: state.read().busy, onclick: move |_| {
                        let mut value=draft.peek().clone();value["enabled"]=json!(true);
                        let client=client.clone();
                        spawn(async move {state.write().busy=true;match client.invoke(1,"application:set-turbo-key-setting",json!([value])).await {Ok(value)=>{state.write().turbo=value;state.write().busy=false;close_modal(client,state);},Err(e)=>{state.write().error=e;state.write().busy=false;}}});
                    }, "설정 완료" }
                }
            }
        }
    }
}

fn toggle_key(mut draft: Signal<Value>, code: u64) {
    let mut value = draft.write();
    let mut keys = value["keys"].as_array().cloned().unwrap_or_default();
    if keys.contains(&json!(code)) {
        keys.retain(|v| *v != json!(code));
    } else {
        keys.push(json!(code));
    }
    value["keys"] = json!(keys);
}

#[component]
fn Keycap(spec: Value, draft: Signal<Value>) -> Element {
    if let Some(spacer) = spec["spacer"].as_f64() {
        return rsx! {span{class:"turbo-key-spacer",style:"--key-units:{spacer}"}};
    }
    let code = spec["code"].as_u64().unwrap_or(0);
    let selected = draft.read()["keys"]
        .as_array()
        .is_some_and(|v| v.contains(&json!(code)));
    let style = if spec["row"].is_number() {
        format!(
            "grid-row:{} / span {};grid-column:{} / span {}",
            spec["row"],
            spec["rowSpan"].as_u64().unwrap_or(1),
            spec["column"],
            spec["columnSpan"].as_u64().unwrap_or(1)
        )
    } else {
        format!("--key-units:{}", spec["units"].as_f64().unwrap_or(1.0))
    };
    rsx! {button{class:format!("turbo-keycap {} {}",if selected{"selected"}else{""},if spec["groupStart"]==true{"group-start"}else{""}),style,disabled:spec["disabled"]==true,aria_pressed:selected,onclick:move |_|{if code!=0{toggle_key(draft,code);}}, {spec["label"].as_str().unwrap_or("")}}}
}

#[component]
fn InputSettings() -> Element {
    let state = use_context::<Signal<State>>();
    let mut draft = use_signal(|| state.peek().input.clone());
    let client = use_context::<Client>();
    rsx! {
        h2 { "입력 보호 · 커서" }
        label { input { r#type: "checkbox", checked: draft.read()["enabled"] == true, onchange: move |e| draft.write()["enabled"] = json!(e.checked()) } "Alt+Enter 방지" }
        p { "마비노기 플레이 중 전체 화면 전환 단축키 Alt+Enter 입력을 차단합니다" }
        label { "커서 크기" select { value: draft.read()["cursorScalePercent"].to_string(), onchange: move |e| draft.write()["cursorScalePercent"] = json!(e.value().parse::<u32>().unwrap_or(100)),
            for percent in (25..=800).step_by(25) { option { value: "{percent}", "{percent}%" } }
        } }
        label { "휠로 크기 변경" select { value: draft.read()["cursorWheelModifier"].as_str().unwrap_or("disabled").to_owned(), onchange: move |e| draft.write()["cursorWheelModifier"] = json!(e.value()),
            option { value: "disabled", "사용 안 함" }
            option { value: "control", "Ctrl + 휠" }
            option { value: "alt", "Alt + 휠" }
        } }
        button { disabled: state.read().busy, onclick: move |_| command(client.clone(), state, "application:set-input-guard-setting", json!([draft.peek().clone()])), "저장" }
    }
}

#[component]
fn CpuSettings() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let s = state.read().clone();
    let setting = s.services["affinity"]["gameCoreSetting"].clone();
    let max = setting["maxGameCoreCount"].as_u64().unwrap_or(0).min(256);
    let selected = setting["gameCoreCount"].as_u64().unwrap_or(0);
    let p = setting["performanceCoreCount"].as_u64();
    let e = setting["efficiencyCoreCount"].as_u64();
    let applying = s.pending.contains("optimization:set-game-cpu-core-count");
    rsx! {
        div {
            h2 { "마비노기 CPU 우선 점유 비율 설정" }
            p { if setting["hybrid"] == true { "마비노기에 우선 배정할 P코어 개수를 선택합니다" } else { "마비노기에 우선 배정할 물리 코어 개수를 선택합니다" } }
        }
        if max > 0 {
            div { class: if applying { "game-cpu-core-controls applying" } else { "game-cpu-core-controls" }, aria_label: "마비노기 CPU 코어 개수",
                for core in 1..=max {
                    button { class: if core <= selected { "game-cpu-core-option allocated" } else { "game-cpu-core-option" },
                        style: format!("--allocation-color:hsl({} 58% 42%)", setting["defaultGameCoreCount"].as_u64().map(|default|core_hue(core,max,default)).unwrap_or(0)),
                        aria_pressed: core <= selected,
                        disabled: s.busy || s.services["affinity"]["cpuReorder"]["state"] == "running",
                        onclick: {let client=client.clone(); move |_| command(client.clone(), state, "optimization:set-game-cpu-core-count", json!([core]))}, "{core}"
                    }
                }
                if let (Some(p),Some(e))=(p,e) {
                    if p>1 || e>0 { span { class: "game-cpu-unavailable-label", "선택 불가 코어" } }
                    if p>1 { button { class: "game-cpu-unavailable-core", disabled: true, title: "게임도 필요할 때 사용하며 백그라운드와 입력 프로그램이 공유하는 P-core", "P{p}" } }
                    for core in 1..=e.min(256) { button { class: "game-cpu-unavailable-core", disabled: true, title: "마비노기에 배정하지 않는 E-core", "E" {(p+core).to_string()} } }
                }
            }
            small { class: "game-cpu-core-hint", "선택한 코어 개수가 많을수록 마비노기가 더 많은 CPU를 활용하지만, 터보 키의 입력 성능과 다른 프로그램들의 성능이 저하될 수 있습니다" }
        } else {
            div { class: "cpu-topology-unavailable",
                span { {setting["failureReason"].as_str().unwrap_or("물리 CPU 코어 구성을 확인할 수 없습니다")} }
                if let Some(detail)=setting["failureDetail"].as_str() { small { "상세: {detail}" } }
                Action { label: "다시 확인", class: "developer-tool-secondary", channel: "optimization:refresh-game-cpu-core-setting" }
            }
        }
    }
}

fn creator_statistics(profile: &Value) -> String {
    fn grouped(n: u64) -> String {
        let s = n.to_string();
        let len = s.len();
        s.chars()
            .enumerate()
            .fold(String::new(), |mut out, (i, c)| {
                if i > 0 && (len - i) % 3 == 0 {
                    out.push(',');
                }
                out.push(c);
                out
            })
    }
    match (
        profile["subscriberCount"].as_u64(),
        profile["videoCount"].as_u64(),
    ) {
        (Some(subscribers), Some(videos)) => format!(
            "구독자 {}명 · 동영상 {}개",
            grouped(subscribers),
            grouped(videos)
        ),
        _ => "YouTube 채널".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blackbox_runtime_patch_preserves_feature_and_recording_settings() {
        let mut state = State {
            blackbox: json!({"featureEnabled":true,"enabled":true,"quality":"1080p"}),
            ..Default::default()
        };
        apply_event(
            &mut state,
            "application:blackbox-status-changed",
            &json!({"durationSeconds":120,"bytesUsed":4096}),
        );
        assert_eq!(state.blackbox["featureEnabled"], true);
        assert_eq!(state.blackbox["enabled"], true);
        assert_eq!(state.blackbox["quality"], "1080p");
        assert_eq!(state.blackbox["durationSeconds"], 120);
        assert!(!state.startup_data_ready);
    }
    #[test]
    fn backend_status_keeps_successes_and_serialized_failures() {
        let result = unwrap_status(
            json!({"affinity":{"ok":true,"data":{"running":true}},"graphics":{"ok":false,"error":{"message":"드라이버 접근 거부","code":"EACCES"}}}),
        );
        assert_eq!(result["affinity"]["running"], true);
        assert_eq!(result["graphics"]["error"], "드라이버 접근 거부");
    }
    #[test]
    fn asynchronous_notices_never_replace_exit_confirmation() {
        let mut state = State {
            modal: "close".into(),
            ..Default::default()
        };
        apply_event(
            &mut state,
            "application:notice-available",
            &json!({"id":"notice","markdown":"새 공지"}),
        );
        apply_event(
            &mut state,
            "application:report-responses-available",
            &json!([{"responseId":"report","message":"답변"}]),
        );
        assert_eq!(state.modal, "close");
        assert_eq!(state.notice["id"], "notice");
        assert_eq!(state.reports[0]["responseId"], "report");
    }
    #[test]
    fn report_queue_survives_empty_rechecks_duplicates_and_late_delivery() {
        let mut state = State::default();
        let event = "application:report-responses-available";
        apply_event(&mut state, event, &json!([{"responseId":"first"},{"responseId":"second"}]));
        apply_event(&mut state, event, &json!([]));
        apply_event(&mut state, event, &json!([{"responseId":"second"},{"responseId":"third"}]));
        assert_eq!(state.reports.as_array().unwrap().len(), 3);
        assert_eq!(state.reports[0]["responseId"], "first");
        state.reports.as_array_mut().unwrap().remove(0);
        apply_event(&mut state, event, &json!([{"responseId":"first"}]));
        assert_eq!(state.reports.as_array().unwrap().len(), 2);
        assert_eq!(state.reports[0]["responseId"], "second");
    }
    #[test]
    fn personal_report_has_priority_over_general_notice() {
        let mut state = State::default();
        apply_event(
            &mut state,
            "application:notice-available",
            &json!({"id":"notice","markdown":"공지"}),
        );
        apply_event(
            &mut state,
            "application:report-responses-available",
            &json!([{"responseId":"report"}]),
        );
        apply_event(
            &mut state,
            "application:notice-available",
            &json!({"id":"notice2","markdown":"다음 공지"}),
        );
        assert_eq!(state.modal, "report");
    }
}

fn core_hue(core: u64, max: u64, default: u64) -> u64 {
    if core <= default {
        if default > 1 {
            120 * (core - 1) / (default - 1)
        } else {
            120
        }
    } else if max > default {
        120 * (max - core) / (max - default)
    } else {
        120
    }
}

fn creator_toggle(mut state: Signal<State>) {
    if matches!(state.peek().creator_phase.as_str(), "opening" | "closing") {
        return;
    }
    let opening = state.peek().tab.is_empty();
    {
        let mut s = state.write();
        s.creator_phase = if opening { "opening" } else { "closing" }.into();
        if opening {
            s.tab = "developer".into();
        }
    }
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(780)).await;
        let mut s = state.write();
        s.creator_phase = if opening { "open" } else { "home" }.into();
        if !opening {
            s.tab.clear();
        }
    });
}

fn creator_tab(mut state: Signal<State>, tab: &'static str) {
    if state.peek().tab == tab || !state.peek().tab_phase.is_empty() {
        return;
    }
    state.write().tab_phase = "tab-leaving".into();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
        {
            let mut s = state.write();
            s.tab = tab.into();
            s.tab_phase = "tab-entering".into();
        }
        let _ = document::eval(
            "window.nogiremCreatorScroller?.reset();document.querySelector('.creator-content')?.scrollTo(0,0)",
        );
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
        state.write().tab_phase.clear();
    });
}

#[component]
fn StatusIcon(
    #[props(default = false)] ready: bool,
    #[props(default = false)] checking: bool,
) -> Element {
    rsx! { svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: if ready { "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9Z" } else if checking { "M12 4V1L8 5l4 4V6a6 6 0 0 1 5.65 8H19.7A8 8 0 0 0 12 4Zm-5.65 6H4.3A8 8 0 0 0 12 20v3l4-4-4-4v3a6 6 0 0 1-5.65-8Z" } else { "M1 21h22L12 2 1 21Zm12-3h-2v-2h2v2Zm0-4h-2v-4h2v4Z" } } } }
}

#[component]
fn StatusText(text: ReadSignal<String>) -> Element {
    let state = use_context::<Signal<State>>();
    let mut shown = use_signal(|| "공개 사용자 버전".to_owned());
    let mut from = use_signal(String::new);
    let mut phase = use_signal(|| "done");
    let mut generation = use_signal(|| 0u64);
    use_effect(move || {
        let next = text();
        if next != *shown.peek() {
            let previous = shown.peek().clone();
            from.set(previous);
            shown.set(next);
            phase.set("enter");
            generation += 1;
        }
    });
    rsx! {
        span { class: format!("boost-text-final {} {} {}", if !state.read().visual_active || state.read().document_hidden { "animation-paused" } else { "" }, if phase() == "enter" { "concealed" } else { "" }, if shown() == "실시간 부스트중" && phase() == "done" { "boosting" } else { "" }), "{shown}" }
        if shown() == "실시간 부스트중" && phase() == "done" && state.read().visual_active && !state.read().document_hidden {
            span { class: "boost-progress", "aria-hidden": "true", span {} span {} span {} span {} }
        }
        if phase() == "enter" { span { key: "base-{generation}", class: if from().contains("중단") { "boost-text-base transitioning paused-source" } else { "boost-text-base transitioning" }, "{from}" } }
        if phase() != "done" { span { key: "over-{generation}", class: if phase() == "leave" { "boost-text-over leaving" } else { "boost-text-over" }, onanimationend: move |_| { if phase() == "enter" { phase.set("leave"); } else { phase.set("done"); } }, "{shown}" } }
    }
}

fn dxvk_class(affinity: &Value) -> &'static str {
    if affinity["renderer"]["mode"] == "direct3d9" {
        return "warning";
    }
    match affinity["dxvk"]["state"].as_str().unwrap_or("") {
        "latest" | "applied-unverified" => "ready",
        "update-required" | "incompatible" | "unavailable" => "warning",
        _ => "checking",
    }
}
fn dxvk_label(affinity: &Value) -> &'static str {
    if affinity["renderer"]["mode"] == "direct3d9" {
        return "Vulkan을 사용중이지 않음";
    }
    match affinity["dxvk"]["state"].as_str().unwrap_or("") {
        "latest" => "Vulkan 최신버전 사용중",
        "applied-unverified" => "Vulkan 적용됨 · 최신 확인 불가",
        "update-required" => "Vulkan 업데이트가 필요함",
        "incompatible" => "Vulkan GPU 드라이버 호환 필요",
        "unavailable" => "DXVK 상태 확인 불가",
        _ => "DXVK 확인 중",
    }
}

#[component]
fn BlackboxControls() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let initial = state.peek().blackbox["enabled"] == true;
    let mut shown = use_signal(|| initial);
    let mut target = use_signal(|| initial);
    let mut from = use_signal(|| initial);
    let mut phase = use_signal(|| "done");
    let mut generation = use_signal(|| 0u64);
    use_effect(move || {
        let enabled = state.read().blackbox["enabled"] == true;
        if enabled != *target.peek() {
            let previous = *shown.peek();
            from.set(previous);
            target.set(enabled);
            phase.set("enter");
            generation += 1;
        }
    });
    let snapshot = state.read().clone();
    let minutes = (snapshot.blackbox["durationSeconds"]
        .as_f64()
        .unwrap_or(0.0)
        .max(0.0)
        / 60.0)
        .floor() as u64;
    let duration = if minutes >= 60 {
        format!("{}시간 {}분", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}분")
    };
    rsx! {
        div { class: if snapshot.controls_entered { "blackbox-main-controls entered" } else { "blackbox-main-controls" },
            div { class: "blackbox-main-status",
                button { class: if shown() { "blackbox-main-link active" } else { "blackbox-main-link" }, disabled: snapshot.busy || phase() != "done", aria_pressed: snapshot.blackbox["enabled"] == true,
                    aria_label: if snapshot.blackbox["enabled"] == true { "블랙박스 끄기" } else { "블랙박스 켜기" },
                    onclick: {let client=client.clone();move |_| command(client.clone(),state,"application:set-blackbox-enabled",json!([state.peek().blackbox["enabled"] != true]))},
                    span { class: if phase() == "enter" { "blackbox-text-final concealed" } else { "blackbox-text-final" }, if shown() { "블랙박스 켜짐" } else { "블랙박스 꺼짐" } }
                    if phase() == "enter" { span { key: "base-{generation}", class: if from() { "blackbox-text-base active-source" } else { "blackbox-text-base" }, if from() { "블랙박스 켜짐" } else { "블랙박스 꺼짐" } } }
                    if phase() != "done" { span { key: "over-{generation}", class: if phase() == "leave" { "blackbox-text-over leaving" } else { "blackbox-text-over" }, "aria-hidden": "true", if target() { "블랙박스 켜짐" } else { "블랙박스 꺼짐" } } }
                }
                span { class: if shown() { "blackbox-main-duration active" } else { "blackbox-main-duration" }, "{duration}" }
            }
            button { class: if shown() { "blackbox-window-link active" } else { "blackbox-window-link" }, aria_label: "게임 블랙박스 관리 창 열기", onclick: move |_| command(client.clone(),state,"application:open-blackbox-manager",json!([])),
                svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: "M4 6h10a2 2 0 0 1 2 2v2.2l4-2.4a1 1 0 0 1 1.5.86v6.68a1 1 0 0 1-1.5.86l-4-2.4V16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z" } }
                if phase() != "done" { span { key: "mask-{generation}", class: if phase() == "leave" { "blackbox-icon-mask leaving" } else { "blackbox-icon-mask" }, "aria-hidden": "true", onanimationend: move |_| { if phase() == "enter" { shown.set(target()); phase.set("leave"); } else { phase.set("done"); } } } }
            }
        }
    }
}

#[component]
fn TermsDialog() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let mount_scroller = move |_| { spawn(async move {
        let _ = document::eval(r#"
            const {createSmoothWheelScroller} = await import('/web/smooth-wheel-scroll.mjs');
            window.nogiremTermsScroller?.dispose();
            const element = document.querySelector('.terms-scroll');
            if (element) {
                const scroller = createSmoothWheelScroller(() => element.isConnected ? element : null);
                const wheel = event => scroller.handleWheel(event);
                element.addEventListener('wheel', wheel, {passive:false});
                window.nogiremTermsScroller = {dispose() {
                    scroller.stop();
                    element.removeEventListener('wheel', wheel);
                }};
            }
        "#).await;
    }); };
    use_drop(|| { let _ = document::eval("window.nogiremTermsScroller?.dispose(); delete window.nogiremTermsScroller;"); });
    rsx! {
        div { class: "terms-backdrop",
            div { class: "terms-modal", role: "dialog", aria_modal: "true", aria_labelledby: "terms-modal-title",
                header { h2 { id: "terms-modal-title", "터보 키 다운로드 전 확인" }
                    button { class: "terms-close", aria_label: "약관 닫기", disabled: state.read().busy, onclick: {let client=client.clone();move |_| close_modal(client.clone(),state)}, CloseIcon {} }
                }
                div { class: "terms-scroll", onmounted: mount_scroller, div { class: "introduce-markdown turbo-terms-content", dangerous_inner_html: crate::markdown::blocks(include_str!("../../TURBO_KEY_TERMS.md"),false,false) } }
                footer { div { class: "turbo-terms-footer", div { class: "turbo-terms-actions",
                    button { class: "developer-tool-secondary", disabled: state.read().busy, onclick: move |_| close_modal(client.clone(),state), "취소" }
                    Action { label: if state.read().busy { "다운로드 중…" } else { "확인 후 다운로드" }, channel: "application:download-turbo-key-helper", class: "" }
                } } }
            }
        }
    }
}

#[component]
fn NoticeDialog() -> Element {
    let state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let scroll_key = use_hook(move || {
        let s = state.peek();
        format!("{}:{}", s.modal, if s.modal == "report" { &s.reports[0]["responseId"] } else { &s.notice["id"] })
    });
    let content_key = use_memo(move || {
        let s = state.read();
        format!("{}:{}", s.modal, if s.modal == "report" { &s.reports[0]["responseId"] } else { &s.notice["id"] })
    });
    use_effect(move || {
        let key = serde_json::to_string(&content_key()).unwrap();
        let _ = document::eval(&format!("requestAnimationFrame(()=>{{const s=window.nogiremNoticeScroller;if(s && s.contentKey!=={key}){{s.contentKey={key};s.reset();document.querySelectorAll('.notice-backdrop,.notice-dialog').forEach(e=>e.classList.remove('closing'));}}}});"));
    });
    let mount_key = scroll_key.clone();
    let mount_scroller = move |_| {
        let key = serde_json::to_string(&mount_key).unwrap();
        spawn(async move {
            let script = format!(r#"
                const key = {key};
                const element = document.querySelector('.notice-content');
                const {{createSmoothWheelScroller}} = await import('/web/smooth-wheel-scroll.mjs');
                if (element?.isConnected) {{
                    window.nogiremNoticeScroller?.dispose();
                    element.scrollTop = 0;
                    const scroller = createSmoothWheelScroller(() => element.isConnected ? element : null);
                    const wheel = event => {{ if (event.deltaY && !event.shiftKey) scroller.handleWheel(event); }};
                    const stop = () => scroller.stop();
                    element.addEventListener('wheel', wheel, {{passive:false}});
                    element.addEventListener('pointerdown', stop);
                    element.addEventListener('keydown', stop);
                    window.nogiremNoticeScroller = {{key, contentKey:key, reset() {{scroller.reset(0); element.scrollTop=0;}}, dispose() {{
                        scroller.stop();
                        element.removeEventListener('wheel', wheel);
                        element.removeEventListener('pointerdown', stop);
                        element.removeEventListener('keydown', stop);
                    }}}};
                }}
            "#);
            let _ = document::eval(&script).await;
        });
    };
    use_drop(move || {
        let key = serde_json::to_string(&scroll_key).unwrap();
        let _ = document::eval(&format!("if(window.nogiremNoticeScroller?.key==={key}){{window.nogiremNoticeScroller.dispose();delete window.nogiremNoticeScroller;}}"));
    });
    let s = state.read().clone();
    let markdown = s.notice["markdown"].as_str().unwrap_or("");
    let title = if s.modal == "report" {
        s.reports[0]["title"].as_str().unwrap_or("버그 리포트 답변")
    } else {
        markdown
            .lines()
            .find_map(|line| line.trim().strip_prefix("# "))
            .unwrap_or("공지사항")
    };
    rsx! {
        div { class: "notice-backdrop", div { class: "notice-dialog", role: "alertdialog", aria_modal: "true", aria_label: title,
            div { class: "notice-content", onmounted: mount_scroller,
                if s.modal == "report" {
                    div { class: "report-response-content",
                        div { class: "report-response-heading", span { "BUG REPORT RESPONSE" } time { {s.reports[0]["answeredAt"].as_str().unwrap_or("").split('T').next().unwrap_or("")} } }
                        h2 { "{title}" } p { {s.reports[0]["message"].as_str().unwrap_or("")} }
                        dl { div { dt { "리포트 고유값" } dd { {s.reports[0]["reportId"].as_str().unwrap_or("")} } }
                            if let Some(version)=s.reports[0]["minimumVersion"].as_str() { div { dt { "반영 버전" } dd { "{version}" } } }
                        }
                        button { r#type: "button", class: "report-response-confirm", onclick: move |_| close_modal(client.clone(),state), "확인했습니다" }
                    }
                } else {
                    div { class: "application-notice-content", h2 { class: "application-notice-title", "{title}" }
                        RawMarkdown { text: markdown.to_owned(), notice: true, skip_title: true }
                        button { r#type: "button", class: "application-notice-confirm", onclick: move |_| close_modal(client.clone(),state), "확인" }
                    }
                }
            }
        } }
    }
}

#[component]
fn CloseIcon() -> Element {
    rsx! { svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: "M5.3 4 12 10.7 18.7 4 20 5.3 13.3 12l6.7 6.7-1.3 1.3-6.7-6.7L5.3 20 4 18.7l6.7-6.7L4 5.3 5.3 4Z" } } }
}

#[component]
fn RawMarkdown(
    text: String,
    #[props(default = false)] notice: bool,
    #[props(default = false)] skip_title: bool,
) -> Element {
    // display:contents avoids introducing a layout box around the original blocks.
    rsx! { div { style: "display:contents", dangerous_inner_html: crate::markdown::blocks(&text,notice,skip_title) } }
}

#[component]
fn VersionHistory() -> Element {
    let entries = use_hook(|| crate::markdown::history(include_str!("../../VERSION_HISTORY.md")));
    rsx! { div { class: "version-history", for (version,changes) in entries { article { h2 { "{version}" } ul { for change in changes { li { "{change}" } } } } } } }
}

fn update_available(update:&Value)->bool {
    matches!(update["phase"].as_str(),Some("available"|"downloading"|"downloaded"|"installing"|"error")) && update["version"].as_str().is_some_and(|v|!v.is_empty())
}
#[component]
fn UpdatePreview()->Element {
    let mut state=use_context::<Signal<State>>();let client=use_context::<Client>();let s=state.read();
    let downloaded=matches!(s.update["phase"].as_str(),Some("downloaded"|"installing"));
    let failed=s.update["phase"]=="error";
    let installing=s.update["phase"]=="installing" || s.busy;
    let percent=s.update["percent"].as_f64().unwrap_or(0.).clamp(0.,100.);
    let error=s.update["error"].as_str().unwrap_or("업데이트 다운로드에 실패했습니다");
    rsx! {div {class:"update-preview-overlay",role:"dialog",aria_modal:"true",aria_label:"업데이트",
        section {class:"update-preview-panel",
            button {class:"update-dismiss",r#type:"button",onclick:move |_| state.write().update_dismissed=true,"숨기기"}
            h2 {if failed {"업데이트를 완료하지 못했습니다"} else if downloaded {"새 버전 다운로드 완료됨"} else {"새 버전을 가져오고 있습니다"}}
            if failed {p {"{error}"} button {r#type:"button",disabled:s.busy,onclick:move |_| command(client.clone(),state,"application:check-update",json!([])),"다시 시도"}}
            else if downloaded {button {class:"update-install",r#type:"button",disabled:installing,onclick:move |_| command(client.clone(),state,"application:install-update",json!([])),if installing {"설치 준비 중"} else {"새 버전 설치"}}}
            else {div {class:"update-progress",aria_live:"polite",span {"{percent.floor()}%"} div {class:"update-progress-track",div {class:"update-progress-value",style:"width: {percent}%"}}}}
        }
    }}
}

fn creator_prompt_visible(s:&State)->bool {
    s.interface_visible && s.identity=="done" && s.navigation_ready && s.creator_prompt_loaded
        && s.creator_prompt && s.tab.is_empty() && matches!(s.creator_phase.as_str(),""|"home")
        && s.modal.is_empty() && s.visual_active && !s.document_hidden
}
#[component]
fn CreatorPrompt(visible:bool)->Element {
    let mut mounted=use_signal(||false);let mut leaving=use_signal(||false);let mut generation=use_signal(||0u64);
    use_effect(use_reactive!(|(visible,)| {
        let id=*generation.peek()+1;generation.set(id);
        if visible {mounted.set(true);leaving.set(false);}
        else if *mounted.peek() {leaving.set(true);spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
            if *generation.peek()==id {mounted.set(false);}
        });}
    }));
    rsx! {if mounted() {span {class:if leaving() {"creator-prompt leaving"} else {"creator-prompt"},"고급 기능은 여기에서"}}}
}
