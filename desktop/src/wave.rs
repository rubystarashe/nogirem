use crate::{bridge::Client, ui::State};
use dioxus::prelude::*;
use serde_json::json;

#[component]
pub fn Wave() -> Element {
    let mut state = use_context::<Signal<State>>();
    let client = use_context::<Client>();
    let mut startup = use_signal(|| {
        !std::env::args().any(|arg| arg == "--smoke-test" || arg == "--startup-tray")
            || std::env::args().any(|arg| arg == "--smoke-startup")
    });
    use_future(move || {
        let client = client.clone();
        async move {
            let setup = format!(
                r#"
                {}
                window.nogiremWave?.dispose();
                window.nogiremWave = window.createNogiremWave(document.getElementById('nogirem-wave'), (event,args) => dioxus.send({{event,args}}));
                window.nogiremApplyWave = () => {{
                    const config = window.nogiremWaveConfig;
                    if (!config) return;
                    window.nogiremWave.setStartupMuted(config.muted);
                    window.nogiremWave.setLogoVisible(config.home);
                    if (config.ready && !config.transitioning) window.nogiremWave.setPaused(config.paused);
                    window.nogiremWave.setAmbientEnabled(config.active);
                    window.nogiremWave.setPageVisible(config.visible);
                    if (config.ready && !window.nogiremWaveStarted) {{
                        window.nogiremWaveStarted = true;
                        if (config.skip) window.nogiremWave.skipStartup();
                        else window.nogiremWave.allowStartup();
                    }}
                }};
                window.nogiremApplyWave();
                await new Promise(() => {{}});
            "#,
                include_str!("wave.js")
            );
            let mut eval = document::eval(&setup);
            while let Ok(value) = eval.recv::<serde_json::Value>().await {
                match value["event"].as_str() {
                    Some("onplaybackstart") => {
                        let _ = client
                            .invoke(1, "application:begin-startup-reveal", json!([]))
                            .await;
                    }
                    Some("onstartupcomplete") => {
                        let _ = document::eval("window.nogiremWave.finishStartup()").await;
                    }
                    Some("onstartuphidden") => {
                        startup.set(false);
                        state.write().interface_visible = true;
                        let skip = std::env::args()
                            .any(|a| a == "--startup-tray" || a == "--smoke-test")
                            && !std::env::args().any(|a| a == "--smoke-startup");
                        if skip {
                            let mut s = state.write();
                            s.identity = "done".into();
                            s.controls_entered = true;
                            s.navigation_ready = true;
                        } else {
                            state.write().identity = "brand".into();
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                state.write().identity = "transition".into();
                            });
                        }
                        if std::env::args().any(|a| a == "--smoke-test") {
                            let _ = client
                                .invoke(1, "application:begin-startup-reveal", json!([]))
                                .await;
                        }
                    }
                    Some("onstartupidle") => {
                        let _ = client
                            .invoke(1, "application:complete-startup-animation", json!([]))
                            .await;

                    }
                    _ => {}
                }
            }
        }
    });
    let wave_config = use_memo(move || {
        let state = state.read();
        let config = json!({"ready":state.ready && state.startup_data_ready,"muted":state.muted,
            "paused":!state.boost_action.unwrap_or(state.services["affinity"]["running"] == true && state.services["memory"]["running"] == true),
            "transitioning":state.color_transition.is_some() || state.boost_action.is_some(),
            "home":state.tab.is_empty(),
            "active":state.boost_action.unwrap_or(state.services["affinity"]["running"] == true && state.services["memory"]["running"] == true) && (state.services["affinity"]["gameActive"] == true || state.services["memory"]["gameActive"] == true),
            "visible":state.visual_active && !state.document_hidden,
            "skip":std::env::args().any(|arg| arg == "--smoke-test" || arg == "--startup-tray") && !std::env::args().any(|a| a == "--smoke-startup")});
        config
    });
    use_effect(move || {
        let config = wave_config.read();
        let _ = document::eval(&format!(
            "window.nogiremWaveConfig={config}; window.nogiremApplyWave?.();"
        ));
    });
    use_drop(|| {
        let _ = document::eval("window.nogiremWave?.dispose()");
    });
    rsx! { div { class: if startup() { "game-wave active startup" } else { "game-wave active" }, aria_hidden: "true",
        canvas { id: "nogirem-wave", width: "640", height: "290" }
        if startup() {
            div { class: "game-wave-copy",
                img { class: "game-wave-logo", src: "/public/logo3.png", alt: "", draggable: "false" }
                strong { "마비노기 렘 부스터" }
            }
        }

    } }
}

// Preserve the original brand -> public release label -> boost status sequence.
pub fn finish_identity(mut state: Signal<State>) {
    if state.peek().identity != "transition" {
        return;
    }
    state.write().identity = "version".into();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        state.write().identity = "done".into();
        tokio::time::sleep(std::time::Duration::from_millis(2030)).await;
        state.write().controls_entered = true;
        tokio::time::sleep(std::time::Duration::from_millis(970)).await;
        state.write().navigation_ready = true;
    });
}
