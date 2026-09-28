use crate::{Result, http, powershell, storage};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub const CHANNEL: &str = "https://www.youtube.com/channel/UCb7m0UV734CHm78Mb0zEBHg";
pub fn iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn read(path: &Path) -> Value {
    storage::read_json(path)
        .ok()
        .flatten()
        .unwrap_or(Value::Null)
}
fn matches(pattern: &str, s: &str) -> bool {
    regex::Regex::new(pattern).unwrap().is_match(s)
}
pub fn preference(user: &Path, action: &str, value: &Value) -> Result<Value> {
    let music = user.join("startup-music.json");
    let prompt = user.join("creator/prompt.json");
    match action {
        "get-startup-music-setting" => Ok(json!({"muted":read(&music)["muted"]==true})),
        "set-startup-music-setting" => {
            let muted = value
                .as_bool()
                .ok_or("시작 음악 설정 값이 올바르지 않습니다")?;
            storage::write_json(&music, &json!({"muted":muted,"updatedAt":iso()}))?;
            Ok(json!({"muted":muted}))
        }
        "get-creator-prompt-dismissed" => Ok(json!(read(&prompt)["clickCount"].as_u64().unwrap_or(0)>=2)),
        // Retain the IPC contract for compatibility; merely displaying a tip never records a click.
        "record-creator-prompt-display" => Ok(json!(false)),
        "dismiss-creator-prompt" => {
            let mut old=read(&prompt);if !old.is_object(){old=json!({});}
            old["clickCount"]=json!(old["clickCount"].as_u64().unwrap_or(0).saturating_add(1).min(2));
            old["lastDismissedAt"]=json!(iso());
            storage::write_json(&prompt,&old)?;Ok(json!(true))
        }
        "dismiss-notice" => {
            let id = value.as_str().unwrap_or("");
            if !matches("(?i)^[a-f0-9]{64}$", id) {
                return Ok(json!(false));
            }
            storage::write_json(
                &user.join("notice-dismissed.json"),
                &json!({"id":id,"dismissedAt":iso()}),
            )?;
            Ok(json!(true))
        }
        "acknowledge-report-response" => {
            let id = value.as_str().unwrap_or("");
            if id.is_empty() || id.encode_utf16().count() > 100 {
                return Ok(json!(false));
            }
            let path = user.join("report-response-acknowledgements.json");
            let mut ids = read(&path)["responseIds"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if !ids.contains(value) {
                ids.push(value.clone())
            }
            storage::write_json(
                &path,
                &json!({"schemaVersion":1,"responseIds":ids,"updatedAt":iso()}),
            )?;
            Ok(json!(true))
        }
        _ => Err(format!("Unknown preference: {action}")),
    }
}
pub fn normalize_notice(text: &str) -> Result<Value> {
    let normalized = text.replace("\r\n", "\n");
    let normalized = normalized.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if normalized.is_empty() {
        return Ok(Value::Null);
    }
    if normalized.len() > 1024 * 1024 {
        return Err("공지사항 문서가 허용 크기를 초과했습니다".into());
    }
    Ok(json!({"id":http::sha256(normalized.as_bytes()),"markdown":normalized}))
}
pub fn notice(root: &Path, user: &Path) -> Result<Value> {
    notice_document(root, user, http::bytes(
        "https://raw.githubusercontent.com/rubystarashe/nogirem/main/NOTICE.md",
        1024 * 1024, 10000, &[],
    ))
}
fn notice_document(root: &Path, user: &Path, download: Result<Vec<u8>>) -> Result<Value> {
    let bytes = download.or_else(|_| fs::read(root.join("NOTICE.md")).map_err(|e| e.to_string()))?;
    let n = normalize_notice(&String::from_utf8_lossy(&bytes))?;
    Ok(
        if n.is_null() || n["id"] == read(&user.join("notice-dismissed.json"))["id"] {
            Value::Null
        } else {
            n
        },
    )
}
fn report_id(s: &str) -> bool {
    matches(
        r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        s,
    )
}
pub fn normalize_reports(v: &Value) -> Result<Value> {
    let text = if let Some(s) = v.as_str() {
        s.to_owned()
    } else {
        v.to_string()
    };
    if text.len() > 1024 * 1024 {
        return Err("버그 리포트 답변 문서가 허용 크기를 초과했습니다".into());
    }
    let doc: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if doc["schemaVersion"] != 1 || !doc["responses"].is_array() {
        return Err("REPORT.json 형식이 올바르지 않습니다".into());
    }
    let bounded = |v: &Value, n: usize| {
        v.as_str()
            .is_some_and(|s| !s.is_empty() && s.encode_utf16().count() <= n)
    };
    Ok(
        json!({"schemaVersion":1,"generatedAt":doc["generatedAt"].as_str(),"responses":doc["responses"].as_array().unwrap().iter().filter(|r|report_id(r["reportId"].as_str().unwrap_or(""))&&bounded(&r["responseId"],100)&&r["answeredAt"].is_string()&&bounded(&r["title"],200)&&bounded(&r["message"],10000)).cloned().collect::<Vec<_>>()}),
    )
}
fn timestamp(v: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()
        .map(|d| d.timestamp_millis())
}
pub fn select_reports(doc: &Value, owned: &[Value], ack: &[Value], now: i64) -> Value {
    json!(
        doc["responses"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| owned.contains(&r["reportId"])
                && timestamp(&r["answeredAt"]).is_some()
                && (!r["expiresAt"].is_string()
                    || r["expiresAt"] == ""
                    || timestamp(&r["expiresAt"]).is_some_and(|t| t > now))
                && !ack.contains(&r["responseId"]))
            .cloned()
            .collect::<Vec<_>>()
    )
}
pub fn reports(root: &Path, user: &Path, packaged: bool) -> Result<Value> {
    reports_from(root, user, packaged, "https://raw.githubusercontent.com/rubystarashe/nogirem/main/REPORT.json")
}
fn reports_from(root: &Path, user: &Path, packaged: bool, url: &str) -> Result<Value> {
    let path = user.join("report-response-cache.json");
    let cache = read(&path);
    let cached = normalize_reports(&cache["document"]).ok();
    let doc = if !packaged {
        normalize_reports(&json!(
            fs::read_to_string(root.join("REPORT.json")).map_err(|e| e.to_string())?
        ))?
    } else {
        let download = (|| {
            use std::{io::Read, time::Duration};
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("nogirem")
                .build()
                .map_err(|e| e.to_string())?;
            let mut request = client
                .get(url);
            if let Some(etag) = cache["etag"].as_str() {
                request = request.header("If-None-Match", etag)
            }
            let response = request.send().map_err(|e| e.to_string())?;
            if response.status().as_u16() == 304 {
                return cached.clone().ok_or("보고서 캐시가 없습니다".into());
            }
            let response = response.error_for_status().map_err(|e| e.to_string())?;
            let etag = response
                .headers()
                .get("etag")
                .and_then(|h| h.to_str().ok())
                .map(str::to_owned);
            let mut text = String::new();
            response
                .take(1024 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            let doc = normalize_reports(&json!(text))?;
            storage::write_json(
                &path,
                &json!({"etag":etag.unwrap_or_else(||format!("\"{}\"",http::sha256(text.as_bytes()))),"document":doc,"checkedAt":iso()}),
            )?;
            Ok::<_, String>(doc)
        })();
        download.or_else(|e| cached.ok_or(e))?
    };
    let owned = read(&user.join("report-owned.json"))["reportIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.get("id").cloned())
        .collect::<Vec<_>>();
    let ack = read(&user.join("report-response-acknowledgements.json"))["responseIds"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(select_reports(&doc, &owned, &ack, crate::now_ms() as i64))
}
pub fn own_report(user: &Path, id: &str) -> Result<()> {
    if !report_id(id) {
        return Err("진단 로그 고유값이 올바르지 않습니다".into());
    }
    let path = user.join("report-owned.json");
    let mut ids = read(&path)["reportIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| report_id(r["id"].as_str().unwrap_or("")) && r["createdAt"].is_string())
        .cloned()
        .collect::<Vec<_>>();
    if !ids.iter().any(|r| r["id"] == id) {
        ids.push(json!({"id":id,"createdAt":iso()}))
    }
    storage::write_json(&path, &json!({"schemaVersion":1,"reportIds":ids}))
}
fn channel_entity(v: &Value) -> Option<&Value> {
    if v["name"].is_string()
        && v["alternateName"].is_string()
        && v["image"].is_string()
        && v["interactionStatistic"].is_array()
    {
        return Some(v);
    }
    if let Some(a) = v.as_array() {
        for c in a {
            if let Some(r) = channel_entity(c) {
                return Some(r);
            }
        }
    }
    if let Some(o) = v.as_object() {
        for c in o.values() {
            if let Some(r) = channel_entity(c) {
                return Some(r);
            }
        }
    }
    None
}
pub fn parse_channel(html: &str) -> Result<Value> {
    let pattern = regex::Regex::new(
        r#"(?is)<script[^>]+type=["']application/ld\+json["'][^>]*>(.*?)</script>"#,
    )
    .unwrap();
    let mut entity = None;
    for capture in pattern.captures_iter(html) {
        if let Ok(v) = serde_json::from_str::<Value>(&capture[1]) {
            if let Some(c) = channel_entity(&v) {
                entity = Some(c.clone());
                break;
            }
        }
    }
    let c = entity.ok_or("YouTube 채널 프로필 정보를 찾을 수 없습니다")?;
    let count = |v: &Value| {
        let text = v.as_str().unwrap_or("").replace(',', "");
        let n = v.as_f64().or_else(|| {
            if text.trim().is_empty() {
                Some(0.)
            } else {
                text.parse::<f64>().ok()
            }
        });
        match n {
            Some(n) if n.is_finite() && n.fract() == 0. => json!(n as i64),
            Some(n) if n.is_finite() => json!(n),
            _ => Value::Null,
        }
    };
    let subscribers = c["interactionStatistic"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["interactionType"]["@type"] == "FollowAction")
        .map(|s| count(&s["userInteractionCount"]))
        .unwrap_or(json!(0));
    let video = regex::Regex::new(r#""content":"동영상\s*([\d,]+)개""#)
        .unwrap()
        .captures(html)
        .or_else(|| {
            regex::Regex::new(r#"(?i)"content":"([\d,]+)\s+videos?""#)
                .unwrap()
                .captures(html)
        })
        .map(|c| count(&json!(&c[1])))
        .unwrap_or(json!(0));
    Ok(
        json!({"name":c["name"],"handle":c["alternateName"],"subscriberCount":subscribers,"videoCount":video,"avatarUrl":c["image"].as_str().unwrap().replace("\\u003d","="),"channelUrl":CHANNEL}),
    )
}
pub fn channel(user: &Path) -> Result<Value> {
    let path = user.join("creator/youtube-channel.json");
    let mut cache = read(&path);
    let now = crate::now_ms();
    if cache["checkedAt"]
        .as_u64()
        .is_some_and(|t| now.saturating_sub(t) < 86400000)
    {
        cache["source"] = json!("cache");
        cache["stale"] = json!(false);
        return Ok(cache);
    }
    let fetched = (|| {
        let html = http::bytes(
            &format!("{CHANNEL}?hl=ko"),
            8 * 1024 * 1024,
            15000,
            &[
                ("Accept-Language", "ko-KR,ko;q=0.9,en;q=0.8"),
                ("User-Agent", "Mozilla/5.0"),
            ],
        )?;
        let mut profile = parse_channel(&String::from_utf8_lossy(&html))?;
        let avatar = http::bytes(
            profile["avatarUrl"].as_str().unwrap(),
            1024 * 1024,
            15000,
            &[],
        )?;
        let mime = if avatar.starts_with(b"\x89PNG") {
            "image/png"
        } else if avatar.starts_with(b"RIFF") {
            "image/webp"
        } else {
            "image/jpeg"
        };
        profile["avatarDataUrl"] = json!(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(avatar)
        ));
        profile["checkedAt"] = json!(now);
        storage::write_json(&path, &profile)?;
        profile["source"] = json!("network");
        profile["stale"] = json!(false);
        Ok::<_, String>(profile)
    })();
    match fetched {
        Ok(v) => Ok(v),
        Err(e) if cache.is_object() => {
            cache["source"] = json!("cache");
            cache["stale"] = json!(true);
            cache["error"] = json!(e);
            Ok(cache)
        }
        Err(e) => Err(e),
    }
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
fn startup_state(exe: &Path) -> Result<Value> {
    powershell::json(
        &format!(
            r#"$preference=Get-ItemProperty -Path 'HKCU:\Software\Nogirem' -Name 'StartupTrayEnabled' -ErrorAction SilentlyContinue
$preferred=$null -ne $preference -and $preference.StartupTrayEnabled -eq 1
$task=Get-ScheduledTask -TaskName 'Mabinogi Rem Booster Startup' -ErrorAction SilentlyContinue
if($null -eq $task){{[pscustomobject]@{{exists=$false;enabled=$false;matches=$false;preferred=$preferred}}|ConvertTo-Json -Compress;exit 0}}
$action=@($task.Actions)[0]
[pscustomobject]@{{exists=$true;enabled=$task.State -ne 'Disabled';matches=($action.Execute -ieq {} -and $action.Arguments -eq '--startup-tray');preferred=$preferred}}|ConvertTo-Json -Compress"#,
            quote(&exe.to_string_lossy())
        ),
        15000,
    )
}
fn startup_apply(exe: &Path, enabled: bool) -> Result<()> {
    let script = if enabled {
        format!(
            r#"$ErrorActionPreference='Stop'
New-Item -Path 'HKCU:\Software\Nogirem' -Force|Out-Null
New-ItemProperty -Path 'HKCU:\Software\Nogirem' -Name 'StartupTrayEnabled' -PropertyType DWord -Value 1 -Force|Out-Null
$userId=[Security.Principal.WindowsIdentity]::GetCurrent().Name
$action=New-ScheduledTaskAction -Execute {} -Argument '--startup-tray'
$trigger=New-ScheduledTaskTrigger -AtLogOn -User $userId
$principal=New-ScheduledTaskPrincipal -UserId $userId -LogonType Interactive -RunLevel Highest
$settings=New-ScheduledTaskSettingsSet -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName 'Mabinogi Rem Booster Startup' -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force|Out-Null"#,
            quote(&exe.to_string_lossy())
        )
    } else {
        r#"$ErrorActionPreference='Stop'
$task=Get-ScheduledTask -TaskName 'Mabinogi Rem Booster Startup' -ErrorAction SilentlyContinue
if($null -ne $task){Unregister-ScheduledTask -TaskName 'Mabinogi Rem Booster Startup' -Confirm:$false}
Remove-ItemProperty -Path 'HKCU:\Software\Nogirem' -Name 'StartupTrayEnabled' -ErrorAction SilentlyContinue"#.into()
    };
    powershell::run(&script, 30000, 1024 * 1024)?;
    Ok(())
}
pub fn startup(exe: &Path, packaged: bool, set: Option<bool>) -> Result<Value> {
    if !packaged {
        if set.is_some() {
            return Err("설치된 앱에서만 사용할 수 있습니다".into());
        }
        return Ok(
            json!({"supported":false,"enabled":false,"reason":"설치된 앱에서만 사용할 수 있습니다"}),
        );
    }
    if let Some(enabled) = set {
        startup_apply(exe, enabled)?
    }
    let mut state = startup_state(exe)?;
    let valid = |v: &Value| v["exists"] == true && v["enabled"] == true && v["matches"] == true;
    let was_valid = valid(&state);
    if (state["preferred"] == true || was_valid) && (state["preferred"] != true || !was_valid) {
        if let Err(e) = startup_apply(exe, true) {
            return Ok(
                json!({"supported":true,"enabled":was_valid,"reason":format!("Windows 시작 설정 자동 복구 실패: {e}")}),
            );
        }
        state = startup_state(exe)?
    }
    let enabled = valid(&state);
    if set.is_some_and(|v| v != enabled) {
        return Err("Windows 시작 프로그램 설정을 확인하지 못했습니다".into());
    }
    Ok(
        json!({"supported":true,"enabled":enabled,"reason":if state["exists"]==true&&state["matches"]!=true{json!("등록된 실행 경로가 현재 설치 위치와 다릅니다")}else{Value::Null}}),
    )
}
pub fn dispatch(root: &Path, action: &str, p: &Value) -> Result<Value> {
    let user = PathBuf::from(
        p["userData"]
            .as_str()
            .ok_or("Missing user data directory")?,
    );
    match action {
        "app-notice" => notice(root, &user),
        "app-reports" => reports(root, &user, p["packaged"] == true),
        "app-channel" => channel(&user),
        "app-startup" => startup(
            Path::new(p["exe"].as_str().ok_or("Missing executable")?),
            p["packaged"] == true,
            p["enabled"].as_bool(),
        ),
        "app-preference" => preference(&user, p["action"].as_str().unwrap_or(""), &p["value"]),
        _ => Err(format!("Unknown app operation: {action}")),
    }
}

#[cfg(test)]
mod announcement_tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener};
    struct Fixture(std::path::PathBuf);
    impl Fixture { fn new() -> Self {
        let path=std::env::temp_dir().join(format!("nogirem-announcements-{}",uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap(); Self(path)
    }}
    impl Drop for Fixture { fn drop(&mut self) { let _=fs::remove_dir_all(&self.0); } }
    fn server(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let url=format!("http://{}/document",listener.local_addr().unwrap());
        let thread=std::thread::spawn(move || {
            let mut requests=vec![];
            for (status,body) in responses {
                let (mut stream,_)=listener.accept().unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut request=Vec::new(); let mut buf=[0;1024];
                while !request.windows(4).any(|v|v==b"\r\n\r\n") {
                    let n=stream.read(&mut buf).unwrap(); if n==0 {break;} request.extend_from_slice(&buf[..n]);
                }
                requests.push(String::from_utf8_lossy(&request).to_lowercase());
                write!(stream,"HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nETag: \"fixture\"\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
            requests
        }); (url,thread)
    }
    #[test]
    fn creator_prompt_counts_only_open_clicks_and_stops_after_two() {
        let f=Fixture::new();
        storage::write_json(&f.0.join("creator/prompt.json"),&json!({"displayCount":3})).unwrap();
        for _ in 0..5 {
            assert_eq!(preference(&f.0,"get-creator-prompt-dismissed",&Value::Null).unwrap(),false);
            assert_eq!(preference(&f.0,"record-creator-prompt-display",&Value::Null).unwrap(),false);
        }
        assert!(read(&f.0.join("creator/prompt.json"))["clickCount"].is_null());
        for count in 1..=2 {
            preference(&f.0,"dismiss-creator-prompt",&Value::Null).unwrap();
            let disk=read(&f.0.join("creator/prompt.json"));assert_eq!(disk["clickCount"],json!(count));
            assert_eq!(disk["displayCount"],3);assert!(disk["lastDismissedAt"].is_string());
            assert_eq!(preference(&f.0,"get-creator-prompt-dismissed",&Value::Null).unwrap(),json!(count==2));
        }
        preference(&f.0,"dismiss-creator-prompt",&Value::Null).unwrap();
        assert_eq!(read(&f.0.join("creator/prompt.json"))["clickCount"],2);
    }
    #[test]
    fn notice_dismissal_changed_content_and_offline_fallback() {
        let f=Fixture::new(); fs::write(f.0.join("NOTICE.md"),"# bundled").unwrap();
        let first=notice_document(&f.0,&f.0,Ok(b"# first\r\n".to_vec())).unwrap();
        assert_eq!(first["markdown"],"# first");
        // A second invocation (including after an update/restart) must still return unread content.
        assert!(!f.0.join("notice-dismissed.json").exists());
        assert_eq!(notice_document(&f.0,&f.0,Ok(b"# first\n".to_vec())).unwrap()["id"],first["id"]);
        preference(&f.0,"dismiss-notice",&first["id"]).unwrap();
        assert!(notice_document(&f.0,&f.0,Ok(b"# first\n".to_vec())).unwrap().is_null());
        let changed=notice_document(&f.0,&f.0,Ok(b"# changed".to_vec())).unwrap(); assert_ne!(first["id"],changed["id"]);
        assert_eq!(notice_document(&f.0,&f.0,Err("offline".into())).unwrap()["markdown"],"# bundled");
        assert_eq!(normalize_notice("\u{feff}# first\r\n").unwrap(),first);
    }
    #[test]
    fn report_http_etag_acknowledgement_and_bad_response_cache_fallback() {
        let f=Fixture::new(); let id="ad2dd07f-23a9-40c2-8340-a45fd5c068fb";
        own_report(&f.0,id).unwrap();
        let doc=json!({"schemaVersion":1,"responses":[
            {"reportId":id,"responseId":"first","answeredAt":"2026-09-01T00:00:00Z","title":"답변","message":"내용"},
            {"reportId":id,"responseId":"second","answeredAt":"2026-09-01T00:00:00Z","title":"답변2","message":"내용2"},
            {"reportId":"2290e251-d693-4edc-b738-34fd30cb52d4","responseId":"other","answeredAt":"2026-09-01T00:00:00Z","title":"다른 사용자","message":"제외"}
        ]});
        let (url,thread)=server(vec![(200,doc.to_string()),(304,"".into()),(200,"malformed".into()),(503,"".into())]);
        assert_eq!(reports_from(&f.0,&f.0,true,&url).unwrap().as_array().unwrap().len(),2);
        preference(&f.0,"acknowledge-report-response",&json!("first")).unwrap();
        for _ in 0..3 {
            let remaining=reports_from(&f.0,&f.0,true,&url).unwrap();
            assert_eq!(remaining.as_array().unwrap().len(),1); assert_eq!(remaining[0]["responseId"],"second");
        }
        let requests=thread.join().unwrap(); assert!(requests[1].contains("if-none-match: \"fixture\""));
        // Server has stopped: a new invocation still uses disk cache and acknowledgement.
        assert_eq!(reports_from(&f.0,&f.0,true,&url).unwrap()[0]["responseId"],"second");
        preference(&f.0,"acknowledge-report-response",&json!("second")).unwrap();
        assert_eq!(reports_from(&f.0,&f.0,true,&url).unwrap(),json!([]));
    }
    #[test]
    fn expired_and_invalid_dates_are_never_selected() {
        let id=json!("ad2dd07f-23a9-40c2-8340-a45fd5c068fb");
        let base=json!({"reportId":id,"responseId":"ok","answeredAt":"2026-09-01T00:00:00Z","title":"답변","message":"내용"});
        let mut expired=base.clone(); expired["expiresAt"]=json!("2000-01-01T00:00:00Z");
        let mut invalid=base.clone(); invalid["answeredAt"]=json!("invalid");
        let doc=normalize_reports(&json!({"schemaVersion":1,"responses":[expired,invalid,base]})).unwrap();
        assert_eq!(select_reports(&doc,&[id],&[],crate::now_ms() as i64).as_array().unwrap().len(),1);
    }
}
