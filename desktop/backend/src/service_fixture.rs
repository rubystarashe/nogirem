use crate::{Result, service::Service, settings};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn initial() -> Value {
    let graphics = json!({"supported":true,"detected":true,"allMet":true,"title":"NVIDIA 프로필","gpus":[{"name":"NVIDIA GeForce RTX 4090"}],"goalsList":(["수직 동기화 끄기","최대 프레임 400 FPS","스레드 최적화","최고 성능 선호","저지연 모드 울트라"].iter().map(|label|json!({"label":label,"met":true,"supported":true})).collect::<Vec<_>>())});
    let mut fixture = json!({"status":{"affinity":{"ok":true,"data":{"running":true,"gameActive":false,"dxvk":{"state":"latest"},"characterSimplification":{"applied":true},"gameCoreSetting":{"maxGameCoreCount":4,"gameCoreCount":2,"hybrid":true}}},"memory":{"ok":true,"data":{"running":true}},"graphics":{"ok":true,"data":graphics},"network":{"ok":true,"data":{"optimized":true,"originalStateRecorded":true,"fastPing":{"supported":true,"current":{"interfaceAlias":"이더넷","TcpAckFrequency":1,"TCPNoDelay":1}},"tcpAutoTuning":{"optimized":true}}}},"blackbox":{"enabled":true,"featureEnabled":true,"durationSeconds":125},"turbo":{"enabled":false,"keys":[65],"intervalMs":10,"installed":true,"ignoreInitialDelay":false},"input":{"enabled":false,"cursorScalePercent":100,"cursorWheelModifier":"disabled"},"channelPing":{"enabled":false}});
    if std::env::args().any(|a|a=="--smoke-boost") {
        fixture["status"]["affinity"]["data"]["gameActive"]=json!(true);
        fixture["status"]["memory"]["data"]["gameActive"]=json!(true);
    }
    fixture
}
/// Fixtures are explicitly isolated from production mutations. Only native
/// window operations fall through to the shared controller.
pub fn invoke(s: &Arc<Service>, id: u64, channel: &str, args: &Value) -> Option<Result<Value>> {
    if channel == "optimization:set-frame-boost-enabled" && std::env::args().any(|a|a=="--smoke-boost") {
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    let v = &args[0];
    let mut state = s.fixture_state.lock().unwrap();
    let result = match channel {
        "application:get-launch-context" => {
            json!({"optimizationStatus":state["status"],"startupMusicMuted":!std::env::args().any(|a|a=="--smoke-startup"),"blackboxSetting":state["blackbox"]})
        }
        "application:begin-startup-reveal" => {
            s.windows.command(1, "opacity", json!(1));
            s.windows.focus_main();
            json!(true)
        }
        "application:complete-startup-animation" => json!(true),
        "application:get-startup-tray-setting" => json!({"supported":false,"enabled":false}),
        "application:get-turbo-key-setting" => state["turbo"].clone(),
        "application:get-input-guard-setting" => state["input"].clone(),
        "application:get-channel-ping-setting" => state["channelPing"].clone(),
        "application:set-turbo-key-setting" | "application:set-input-guard-setting" => {
            let key = if channel.contains("turbo-key") {
                "turbo"
            } else {
                "input"
            };
            if key == "turbo"
                && (!v["intervalMs"]
                    .as_u64()
                    .is_some_and(|n| [1, 3, 5, 10, 20, 30].contains(&n))
                    || !v["ignoreInitialDelay"].is_boolean())
            {
                return Some(Err("Invalid turbo setting".into()));
            }
            if let Some(fields) = v.as_object() {
                for (k, v) in fields {
                    state[key][k] = v.clone();
                }
            }
            state[key].clone()
        }
        "optimization:set-game-cpu-core-count" => {
            state["status"]["affinity"]["data"]["gameCoreSetting"]["gameCoreCount"] = v.clone();
            state["status"]["affinity"]["data"].clone()
        }
        "application:get-update-state" => state.get("update").cloned().unwrap_or(json!({"phase":"idle","percent":0})),
        "application:check-update" => {
            if !matches!(state["update"]["phase"].as_str(),Some("downloading"|"downloaded"|"installing")) {
                state["update"]=json!({"phase":"downloading","version":"0.4.1","percent":0});
                s.rpc.event(1,"application:update-state-changed",json!([state["update"]]));
            }
            state["update"].clone()
        },
        "application:install-update" => {state["update"]=json!({"phase":"installing","version":"0.4.1","percent":100});s.rpc.event(1,"application:update-state-changed",json!([state["update"]]));state["update"].clone()},
        "smoke:update-state" => {
            state["update"]=v.clone();s.rpc.event(1,"application:update-state-changed",json!([v]));
            if v["phase"]=="available" {if let Err(e)=s.windows.update_notice(v["version"].as_str().unwrap_or("")){return Some(Err(e));}}
            v.clone()
        },
        "application:get-creator-channel" | "application:get-notice" => Value::Null,
        "application:get-report-responses" | "blackbox-manager:list-clips" => json!([]),
        "application:get-visual-activity" => json!(true),
        "application:dismiss-notice" | "application:acknowledge-report-response" => {
            let key=if channel=="application:dismiss-notice" {"noticeAcks"} else {"reportAcks"};
            state[key]=json!(state[key].as_u64().unwrap_or(0)+1);json!(true)
        },
        "smoke:announcement-acks" => json!({"notice":state["noticeAcks"].as_u64().unwrap_or(0),"report":state["reportAcks"].as_u64().unwrap_or(0)}),
        "application:get-creator-prompt-dismissed" => json!(state["creatorPromptCount"].as_u64().unwrap_or(2)>=2),
        "application:record-creator-prompt-display" => {state["creatorPromptRecords"]=json!(state["creatorPromptRecords"].as_u64().unwrap_or(0)+1);json!(false)},
        "application:dismiss-creator-prompt" => {state["creatorPromptCount"]=json!(state["creatorPromptCount"].as_u64().unwrap_or(0).saturating_add(1).min(2));json!(true)},
        "smoke:creator-prompt-count" => {state["creatorPromptCount"]=v.clone();json!(true)},
        "smoke:creator-prompt-state" => json!({"count":state["creatorPromptCount"],"displayCalls":state["creatorPromptRecords"].as_u64().unwrap_or(0)}),
        "optimization:get-affinity-runtime" => state["status"]["affinity"]["data"].clone(),
        "optimization:get-memory-runtime" => state["status"]["memory"]["data"].clone(),
        "optimization:get-status" => state["status"].clone(),
        "smoke:fail-next-boost" => { state["failBoost"]=json!(true); json!(true) },
        "optimization:set-frame-boost-enabled" => {
            if state["failBoost"]==true {state["failBoost"]=json!(false);return Some(Err("smoke boost failure".into()));}
            let enabled = v["enabled"] == true;
            state["status"]["affinity"]["data"]["running"] = json!(enabled);
            state["status"]["memory"]["data"]["running"] = json!(enabled);
            s.windows.tray_icon(enabled);
            json!({"affinity":state["status"]["affinity"]["data"],"memory":state["status"]["memory"]["data"]})
        }
        "dxvk:get-status" | "dxvk:check-update" => {
            json!({"installed":{"installed":false,"integrity":false},"deployment":{"matchesCurrent":false},"releases":[],"latest":null})
        }
        "blackbox-manager:get-status" => {
            let mut setting = settings::blackbox(&Value::Null);
            for(k,v)in json!({"storageDrives":[],"resolvedQuality":"1080p","running":false,"recording":false}).as_object().unwrap(){setting[k]=v.clone();}
            setting
        }
        "blackbox-manager:get-editor-session" | "blackbox-editor:get-session" => {
            json!({"requestedSeconds":900,"anchorAt":crate::now_ms(),"segments":[],"gaps":[],"timelineDurationSeconds":900})
        }
        "blackbox-manager:report-playback"
        | "blackbox-editor:report-playback"
        | "blackbox-manager:set-page"
        | "blackbox-manager:fit-media" => json!(true),
        "smoke:prewarm-hidden" => {
            drop(state);
            return Some(s.windows.open("dxvk-manager", false).map(|_| json!(true)));
        }
        _ => {
            let action = channel.split_once(':').map(|(_, a)| a).unwrap_or("");
            if (id == 1
                && matches!(
                    channel,
                    "application:minimize-to-tray"
                        | "application:request-close"
                        | "application:confirm-close"
                ))
                || action == "request-close"
                || matches!(
                    channel,
                    "application:open-character-guide"
                        | "application:open-dxvk-guide"
                        | "application:open-dxvk-manager"
                        | "application:open-blackbox-manager"
                        | "application:open-blackbox-editor"
                )
            {
                return None;
            }
            return Some(Err(format!("Unknown fixture channel: {channel}")));
        }
    };
    Some(Ok(result))
}
