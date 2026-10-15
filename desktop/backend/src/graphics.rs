use crate::{Result, command, powershell};
use serde_json::{Value, json};
use std::{collections::HashMap, ffi::c_void, path::Path};
type Handle = *mut c_void;
type NoArg = unsafe extern "C" fn() -> i32;
type SessionArg = unsafe extern "C" fn(Handle) -> i32;
type Create = unsafe extern "C" fn(*mut Handle) -> i32;
type Find = unsafe extern "C" fn(Handle, *const u16, *mut Handle, *mut u8) -> i32;
type Base = unsafe extern "C" fn(Handle, *mut Handle) -> i32;
type Info = unsafe extern "C" fn(Handle, Handle, *mut u8) -> i32;
type Get = unsafe extern "C" fn(Handle, Handle, u32, *mut u8) -> i32;
type GetPrivate = unsafe extern "C" fn(Handle, Handle, u32, *mut u8, *mut u32) -> i32;
type SetPrivate = unsafe extern "C" fn(Handle, Handle, *mut u8, u32, u32) -> i32;
struct Api {
    _library: libloading::Library,
    unload: NoArg,
    create: Create,
    destroy: SessionArg,
    load: SessionArg,
    save: SessionArg,
    find: Find,
    base: Base,
    info: Info,
    get: Get,
    set: Info,
    get_private: Option<GetPrivate>,
    set_private: Option<SetPrivate>,
}
impl Api {
    fn open() -> Result<Self> {
        unsafe {
            let dll = std::path::PathBuf::from(
                std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()),
            )
            .join("System32/nvapi64.dll");
            let library = libloading::Library::new(dll).map_err(|e| e.to_string())?;
            let query = *library
                .get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"nvapi_QueryInterface\0")
                .map_err(|e| e.to_string())?;
            unsafe fn function<T: Copy>(
                q: unsafe extern "C" fn(u32) -> *mut c_void,
                id: u32,
            ) -> Result<T> {
                let p = unsafe { q(id) };
                if p.is_null() {
                    return Err(format!("NVAPI 함수를 찾을 수 없습니다: 0x{id:x}"));
                }
                Ok(unsafe { std::mem::transmute_copy(&p) })
            }
            let init: NoArg = function(query, 0x0150E828)?;
            let api = Self {
                unload: function(query, 0xD22BDD7E)?,
                create: function(query, 0x0694D52E)?,
                destroy: function(query, 0xDAD9CFF8)?,
                load: function(query, 0x375DBD6B)?,
                save: function(query, 0xFCBC7E14)?,
                find: function(query, 0xEEE566B2)?,
                base: function(query, 0xDA8466A0)?,
                info: function(query, 0x61CD6FD6)?,
                get: function(query, 0x73BF8338)?,
                set: function(query, 0x577DD202)?,
                get_private: function(query, 0xEA99498D).ok(),
                set_private: function(query, 0x8A2CF5F5).ok(),
                _library: library,
            };
            check("NVAPI 초기화", init())?;
            Ok(api)
        }
    }
    fn session(&self) -> Result<Session<'_>> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            check("DRS 세션 생성", (self.create)(&mut handle))?;
        }
        let session = Session { api: self, handle };
        unsafe {
            check("DRS 설정 로드", (self.load)(handle))?;
        }
        Ok(session)
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        unsafe {
            (self.unload)();
        }
    }
}
struct Session<'a> {
    api: &'a Api,
    handle: Handle,
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.destroy)(self.handle);
        }
    }
}
fn check(label: &str, status: i32) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} 실패: NVAPI {status}"))
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn blob(size: usize, version: u32) -> Vec<u8> {
    let mut b = vec![0; size];
    put(&mut b, 0, size as u32 | (version << 16));
    b
}
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes())
}
fn get(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn definitions() -> Vec<Value> {
    serde_json::from_str(include_str!("nvidia-definitions.json"))
        .expect("embedded NVAPI definitions")
}
fn candidates(game: &str) -> Vec<String> {
    let mut v = vec![];
    if !game.is_empty() {
        v.push(game.into())
    }
    let name = Path::new(if game.is_empty() { "Client.exe" } else { game })
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if !v.contains(&name) {
        v.push(name)
    }
    v
}
impl Session<'_> {
    fn profile(&self, game: &str) -> Result<(Handle, Option<String>)> {
        for candidate in candidates(game) {
            let mut p = std::ptr::null_mut();
            let status = unsafe {
                (self.api.find)(
                    self.handle,
                    wide(&candidate).as_ptr(),
                    &mut p,
                    blob(16396, 3).as_mut_ptr(),
                )
            };
            if status == 0 {
                return Ok((p, Some(candidate)));
            }
            if status != -166 {
                check("프로그램 프로필 조회", status)?
            }
        }
        let mut p = std::ptr::null_mut();
        unsafe { check("기본 프로필 조회", (self.api.base)(self.handle, &mut p))? }
        Ok((p, None))
    }
    fn name(&self, p: Handle) -> Value {
        let mut b = blob(4116, 1);
        if unsafe { (self.api.info)(self.handle, p, b.as_mut_ptr()) } != 0 {
            return Value::Null;
        }
        let units: Vec<u16> = b[4..4100]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect();
        json!(String::from_utf16_lossy(&units))
    }
    fn read(&self, p: Handle, d: &Value, unsupported: &HashMap<String, String>) -> Result<Value> {
        let key = d["key"].as_str().unwrap();
        let private = d["privateSetting"] == true;
        let reason = unsupported.get(key).cloned().or_else(|| {
            (private && self.api.get_private.is_none())
                .then(|| "비공개 DRS 조회 API를 지원하지 않습니다".into())
        });
        if let Some(reason) = reason {
            return Ok(
                json!({"key":key,"name":d["name"],"available":false,"value":null,"displayValue":"지원 안 함","location":"현재 드라이버","explicit":false,"reason":reason}),
            );
        }
        let mut b = blob(12320, 1);
        let id = d["id"].as_u64().unwrap() as u32;
        let status = unsafe {
            if private {
                (self.api.get_private.unwrap())(self.handle, p, id, b.as_mut_ptr(), &mut 0)
            } else {
                (self.api.get)(self.handle, p, id, b.as_mut_ptr())
            }
        };
        if status != -160 {
            check(&format!("{} 조회", d["name"].as_str().unwrap()), status)?
        }
        let value = if status == -160 {
            d["defaultValue"].as_u64().unwrap() as u32
        } else {
            get(&b, 8220)
        };
        let location = if status == -160 { 3 } else { get(&b, 4108) };
        let label = d["values"][value.to_string()]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| match key {
                "maxFrameRate" => {
                    if value == 0 {
                        "끄기".into()
                    } else {
                        format!("{value} FPS")
                    }
                }
                "maxPreRenderedFrames" => {
                    if value == 0 {
                        "3D 응용 프로그램 설정 사용".into()
                    } else {
                        value.to_string()
                    }
                }
                _ => format!("0x{value:08x}"),
            });
        let mut result = json!({"key":key,"name":d["name"],"available":true,"value":value,"displayValue":label,"location":(["프로그램 프로필","전역 프로필","기본 프로필","드라이버 기본값"].get(location as usize).map(|s|s.to_string()).unwrap_or_else(||format!("알 수 없음({location})"))),"explicit":status != -160});
        if status == -160 {
            result["status"] = json!(status)
        }
        Ok(result)
    }
    fn write(&self, p: Handle, d: &Value) -> i32 {
        let mut b = blob(12320, 1);
        put(&mut b, 4100, d["id"].as_u64().unwrap() as u32);
        put(&mut b, 4104, 0);
        put(&mut b, 8220, d["targetValue"].as_u64().unwrap() as u32);
        unsafe {
            if d["privateSetting"] == true {
                self.api
                    .set_private
                    .map_or(-3, |f| f(self.handle, p, b.as_mut_ptr(), 0, 0))
            } else {
                (self.api.set)(self.handle, p, b.as_mut_ptr())
            }
        }
    }
}
pub fn game_vsync() -> Value {
    let args = [
        "query",
        r"HKCU\Software\Nexon\Mabinogi",
        "/v",
        "VerticalSync",
    ]
    .map(String::from);
    match command::text("reg.exe", &args, 10000) {
        Ok(out) => {
            let re =
                regex::Regex::new(r"(?i)VerticalSync\s+(REG_DWORD|REG_SZ)\s+(0x[0-9a-f]+|\d+)")
                    .unwrap();
            if let Some(c) = re.captures(&out) {
                let raw = c[2].to_lowercase();
                let value = if let Some(hex) = raw.strip_prefix("0x") {
                    u32::from_str_radix(hex, 16)
                } else {
                    raw.parse()
                }
                .unwrap_or(0);
                json!({"available":true,"value":value,"enabled":value!=0,"registryType":c[1].to_uppercase(),"location":r"HKCU\Software\Nexon\Mabinogi\VerticalSync"})
            } else {
                json!({"available":false,"reason":"VerticalSync 레지스트리 값을 찾지 못했습니다"})
            }
        }
        Err(e) => json!({"available":false,"reason":e}),
    }
}
fn set_game_vsync() -> Result<Value> {
    command::text(
        "reg.exe",
        &[
            "add",
            r"HKCU\Software\Nexon\Mabinogi",
            "/v",
            "VerticalSync",
            "/t",
            "REG_SZ",
            "/d",
            "0",
            "/f",
        ]
        .map(String::from),
        10000,
    )?;
    let v = game_vsync();
    if v["available"] != true || v["enabled"] != false {
        return Err("마비노기 수직 동기화 설정 검증에 실패했습니다".into());
    }
    Ok(v)
}
fn nvidia_gpus() -> Vec<Value> {
    command::text(
        "nvidia-smi.exe",
        &[
            "--query-gpu=name,driver_version",
            "--format=csv,noheader,nounits",
        ]
        .map(String::from),
        10000,
    )
    .unwrap_or_default()
    .lines()
    .filter(|s| !s.trim().is_empty())
    .map(|s| {
        let mut p = s.split(',').map(str::trim);
        json!({"name":p.next().unwrap_or(""),"driverVersion":p.next().unwrap_or("")})
    })
    .collect()
}
pub fn nvidia_checks(settings: &[Value], game: &Value) -> Value {
    let value = |k: &str| {
        settings
            .iter()
            .find(|s| s["key"] == k)
            .filter(|s| s["available"] == true)
            .and_then(|s| s["value"].as_u64())
    };
    let (enabled, source) = match value("verticalSync") {
        Some(0x08416747) => (json!(false), "NVIDIA 프로그램 프로필 강제 끄기"),
        Some(0x47814940 | 0x32610244 | 0x71271021 | 0x13245256) => {
            (json!(true), "NVIDIA 프로그램 프로필")
        }
        Some(0x60925292) if game["available"] == true => {
            (game["enabled"].clone(), "마비노기 내부 설정")
        }
        _ => (Value::Null, "판정 불가"),
    };
    let low = value("lowLatencyCpl") == Some(2);
    let driver = value("lowLatencyEnabled") == Some(1);
    let frames = value("maxPreRenderedFrames") == Some(1);
    let max = value("maxFrameRate").filter(|v| *v > 0);
    json!({"verticalSyncEnabled":enabled,"verticalSyncSource":source,"maxFrameRateEnabled":max.is_some(),"maxFrameRate":max,"threadedOptimizationEnabled":value("threadedOptimization")==Some(1),"preferMaximumPerformance":value("powerManagement")==Some(1),"ultraLowLatency":low&&driver&&frames,"ultraLowLatencyCplState":low,"ultraLowLatencyDriverEnabled":driver,"maxPreRenderedFramesOne":frames})
}
const GOALS: [(&str, &str, &[&str]); 5] = [
    ("verticalSyncOff", "수직 동기화 끄기", &["verticalSync"]),
    ("maxFrameRate400", "최대 프레임 400 FPS", &["maxFrameRate"]),
    (
        "threadedOptimizationOn",
        "스레드 최적화",
        &["threadedOptimization"],
    ),
    (
        "preferMaximumPerformance",
        "최고 성능 선호",
        &["powerManagement"],
    ),
    (
        "ultraLowLatency",
        "저지연 모드 울트라",
        &["lowLatencyCpl", "lowLatencyEnabled", "maxPreRenderedFrames"],
    ),
];
fn goal_status(c: &Value, settings: &[Value]) -> Value {
    let met = [
        c["verticalSyncEnabled"] == false,
        c["maxFrameRate"] == 400,
        c["threadedOptimizationEnabled"] == true,
        c["preferMaximumPerformance"] == true,
        c["ultraLowLatency"] == true,
    ];
    let mut goals = json!({"support":{},"allMet":true});
    for (i, (key, _, sources)) in GOALS.iter().enumerate() {
        let supported = sources.iter().all(|source| {
            !settings
                .iter()
                .any(|s| s["key"] == *source && s["available"] == false)
        });
        goals[*key] = json!(met[i]);
        goals["support"][*key] = json!(supported);
        if supported && !met[i] {
            goals["allMet"] = json!(false)
        }
    }
    goals
}
pub fn normalize_nvidia(mut status: Value) -> Value {
    let mut list = vec![];
    for (key, label, sources) in GOALS {
        let error = status["applyFailures"]
            .as_array()
            .and_then(|a| a.iter().find(|f| sources.iter().any(|s| f["key"] == *s)))
            .map(|f| f["reason"].clone())
            .unwrap_or(Value::Null);
        list.push(json!({"key":key,"label":label,"supported":status["goals"]["support"][key]!=false,"met":error.is_null()&&status["goals"][key]==true,"error":error}));
    }
    let detected = status["nvidia"] == true;
    status["detected"] = json!(detected);
    status["vendor"] = if detected {
        json!("nvidia")
    } else {
        Value::Null
    };
    status["vendorLabel"] = if detected {
        json!("NVIDIA")
    } else {
        Value::Null
    };
    status["title"] = json!("NVIDIA 프로필");
    status["scope"] = json!("application");
    status["scopeLabel"] = json!("마비노기 프로그램 프로필");
    status["allMet"] = json!(
        detected
            && list
                .iter()
                .all(|v| v["supported"] == false || v["met"] == true)
    );
    status["goalsList"] = json!(list);
    status
}
pub fn nvidia(game: &str, apply: bool) -> Result<Value> {
    let gpus = nvidia_gpus();
    let game_setting = game_vsync();
    let mut result = json!({"supported":true,"nvidia":!gpus.is_empty(),"gpus":gpus,"gameVerticalSync":game_setting,"settings":[],"checks":{}});
    if gpus.is_empty() {
        if apply {
            return Err("NVIDIA GPU 또는 드라이버를 찾지 못했습니다".into());
        }
        result["reason"] = json!("NVIDIA GPU 또는 드라이버를 찾지 못했습니다");
        return Ok(result);
    }
    let api = match Api::open() {
        Ok(api) => api,
        Err(e) => {
            if apply {
                return Err(e);
            }
            result["reason"] = json!(format!("NVAPI 로드 실패: {e}"));
            return Ok(result);
        }
    };
    let mut unsupported = HashMap::new();
    let mut failures = vec![];
    if apply {
        for d in definitions() {
            for attempt in 0..3 {
                let s = api.session()?;
                let (p, found) = s.profile(game)?;
                if found.is_none() {
                    return Err("마비노기 NVIDIA 프로그램 프로필을 찾지 못했습니다".into());
                }
                let status = s.write(p, &d);
                let key = d["key"].as_str().unwrap();
                if status == -3 {
                    unsupported.insert(
                        key.to_owned(),
                        "현재 드라이버에서 설정 API를 지원하지 않습니다".into(),
                    );
                    break;
                }
                check(&format!("{} 적용", d["name"]), status)?;
                let status = unsafe { (api.save)(s.handle) };
                if status == -3 {
                    unsupported.insert(
                        key.to_owned(),
                        "현재 드라이버에서 설정 저장을 지원하지 않습니다".into(),
                    );
                    break;
                }
                if status == -1 {
                    if attempt < 2 {
                        continue;
                    }
                    failures.push(json!({"key":key,"name":d["name"],"status":status,"reason":format!("{} 저장 실패: NVAPI {status}",d["name"].as_str().unwrap())}));
                    break;
                }
                check(&format!("{} 저장", d["name"]), status)?;
                break;
            }
        }
        result["gameVerticalSync"] = set_game_vsync()?;
    }
    let s = api.session()?;
    let (p, found) = s.profile(game)?;
    let settings = definitions()
        .iter()
        .map(|d| s.read(p, d, &unsupported))
        .collect::<Result<Vec<_>>>()?;
    let checks = nvidia_checks(&settings, &result["gameVerticalSync"]);
    let goals = goal_status(&checks, &settings);
    if apply {
        let unmet: Vec<_> = GOALS
            .iter()
            .filter(|(k, _, sources)| {
                goals[*k] == false
                    && goals["support"][*k] != false
                    && !failures
                        .iter()
                        .any(|f| sources.iter().any(|s| f["key"] == *s))
            })
            .map(|(k, _, _)| *k)
            .collect();
        if !unmet.is_empty() {
            return Err(format!(
                "NVIDIA 최적화 적용 후 목표 설정 검증 실패: {}",
                unmet.join(", ")
            ));
        }
    }
    result["profileFound"] = json!(found.is_some());
    result["matchedApplication"] = json!(found);
    result["profileName"] = s.name(p);
    result["settings"] = json!(settings);
    result["checks"] = checks;
    result["goals"] = goals;
    result["reason"] = Value::Null;
    result["lowLatencyNote"] =
        json!("저지연 모드는 비공개 드라이버 상태값과 최대 사전 렌더링 프레임을 함께 조회합니다");
    if apply {
        result["applyFailures"] = json!(failures)
    }
    Ok(result)
}
pub fn normalize_radeon(raw: &Value, game: Value) -> Result<Value> {
    let gpus = raw["gpus"]
        .as_array()
        .ok_or("Radeon helper가 올바르지 않은 결과를 반환했습니다")?;
    let goals = raw["goals"]
        .as_array()
        .ok_or("Radeon helper가 올바르지 않은 결과를 반환했습니다")?;
    let mut goals: Vec<_> = goals
        .iter()
        .map(|goal| {
            json!({
                "key": goal["key"].as_str().unwrap_or(""),
                "label": goal["label"].as_str().or(goal["key"].as_str()).unwrap_or(""),
                "supported": goal["supported"] == true,
                "met": goal["supported"] == true && goal["met"] == true,
                "currentValue": if goal["currentValue"].is_null() {
                    Value::Null
                } else if goal["currentValue"].is_string() {
                    goal["currentValue"].clone()
                } else {
                    json!(goal["currentValue"].to_string())
                }
            })
        })
        .collect();
    for goal in &mut goals {
        if goal["key"] == "verticalSyncOff" && goal["supported"] == true && !game.is_null() {
            let off = game["available"] == true && game["enabled"] == false;
            goal["met"] = json!(goal["met"] == true && off);
            if !off {
                goal["currentValue"] = json!(format!(
                    "{} · 게임 설정 켜기",
                    goal["currentValue"].as_str().unwrap_or("드라이버 확인")
                ))
            }
        }
    }
    let detected = raw["detected"] == true && !gpus.is_empty();
    let applicable: Vec<_> = goals
        .iter()
        .filter(|goal| goal["supported"] == true)
        .collect();
    Ok(json!({
        "supported": true,
        "detected": detected,
        "amd": detected,
        "vendor": if detected { json!("amd") } else { Value::Null },
        "vendorLabel": if detected { json!("AMD Radeon") } else { Value::Null },
        "title": "AMD Radeon 전역 설정",
        "scope": "global",
        "scopeLabel": "Radeon GPU 전역 설정",
        "persistentGlobal": detected,
        "reason": raw["reason"],
        "gpus": gpus
            .iter()
            .map(|gpu| json!({"name":gpu["name"].as_str().unwrap_or("AMD Radeon GPU")}))
            .collect::<Vec<_>>(),
        "gameVerticalSync": game,
        "goalsList": goals,
        "allMet": detected
            && raw["reason"].is_null()
            && raw["allMet"] == true
            && !applicable.is_empty()
            && applicable.iter().all(|goal| goal["met"] == true)
    }))
}
fn normalize_radeon_devices(raw: &Value) -> Value {
    let devices = raw["gpus"].as_array().cloned().unwrap_or_default();
    let detected = !devices.is_empty();
    let gpus = devices
        .iter()
        .map(|gpu| {
            let name = gpu["name"]
                .as_str()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("AMD Radeon GPU");
            json!({"name":name})
        })
        .collect::<Vec<_>>();
    json!({
        "supported": true,
        "detected": detected,
        "amd": detected,
        "vendor": if detected { json!("amd") } else { Value::Null },
        "vendorLabel": if detected { json!("AMD Radeon") } else { Value::Null },
        "title": if detected { "AMD Radeon 설정 보호" } else { "그래픽 설정" },
        "scope": if detected { json!("read-only") } else { Value::Null },
        "scopeLabel": if detected { json!("Radeon 설정 자동 변경 안 함") } else { Value::Null },
        "persistentGlobal": false,
        "writeProtected": detected,
        "protectionNote": if detected {
            json!("사용자 언더볼팅·오버클럭 프로필을 보호하기 위해 AMD 드라이버 설정을 조회하거나 변경하지 않습니다.")
        } else {
            Value::Null
        },
        "reason": if detected { Value::Null } else { json!("AMD Radeon GPU를 찾지 못했습니다") },
        "gpus": gpus,
        "gameVerticalSync": Value::Null,
        "goalsList": [],
        "allMet": false
    })
}
fn detect_radeon() -> Result<Value> {
    let raw = powershell::json(
        r#"$devices = @(Get-CimInstance Win32_VideoController | Where-Object { $_.PNPDeviceID -match 'VEN_1002' } | ForEach-Object { [ordered]@{ name = $_.Name } })
[ordered]@{ gpus = $devices } | ConvertTo-Json -Depth 4 -Compress"#,
        15000,
    )?;
    Ok(normalize_radeon_devices(&raw))
}
fn run_with(
    game: &str,
    apply: bool,
    detect_amd: impl FnOnce() -> Result<Value>,
    run_nvidia: impl Fn(&str, bool) -> Result<Value>,
) -> Result<Value> {
    let amd = detect_amd().unwrap_or_else(|error| {
        let mut status = normalize_radeon_devices(&json!({"gpus":[]}));
        status["reason"] = json!(format!("AMD Radeon GPU 확인 실패: {error}"));
        status
    });
    if amd["detected"] == true {
        if apply {
            return Err("사용자 언더볼팅·오버클럭 프로필을 보호하기 위해 AMD Radeon 설정 변경을 지원하지 않습니다".into());
        }
        return Ok(amd);
    }
    let nv = run_nvidia(game, false)?;
    if nv["nvidia"] == true {
        return Ok(normalize_nvidia(if apply {
            run_nvidia(game, true)?
        } else {
            nv
        }));
    }
    let reason = [nv["reason"].as_str(), amd["reason"].as_str()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" / ");
    if apply {
        return Err(reason);
    }
    Ok(
        json!({"supported":true,"detected":false,"vendor":null,"vendorLabel":null,"title":"그래픽 설정","scope":null,"scopeLabel":null,"gpus":[],"goalsList":[],"allMet":false,"reason":reason}),
    )
}
pub fn run(_root: &Path, game: &str, apply: bool) -> Result<Value> {
    run_with(game, apply, detect_radeon, nvidia)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn amd_device_detection_is_read_only_and_write_protected() {
        let status = normalize_radeon_devices(&json!({"gpus":[{"name":"AMD Radeon RX 6700 XT"}]}));

        assert_eq!(status["vendor"], "amd");
        assert_eq!(status["writeProtected"], true);
        assert_eq!(status["persistentGlobal"], false);
        assert_eq!(status["allMet"], false);
        assert_eq!(status["goalsList"], json!([]));
    }

    #[test]
    fn missing_amd_device_is_not_detected() {
        let status = normalize_radeon_devices(&json!({"gpus":[]}));

        assert_eq!(status["detected"], false);
        assert_eq!(status["writeProtected"], false);
        assert!(
            status["reason"]
                .as_str()
                .unwrap()
                .contains("찾지 못했습니다")
        );
    }

    #[test]
    fn amd_device_with_missing_name_still_enables_protection() {
        let status = normalize_radeon_devices(&json!({"gpus":[{"name":null}]}));

        assert_eq!(status["detected"], true);
        assert_eq!(status["writeProtected"], true);
        assert_eq!(status["gpus"][0]["name"], "AMD Radeon GPU");
    }

    #[test]
    fn amd_status_never_initializes_nvidia_or_driver_setting_helpers() {
        let calls = Cell::new(0);
        let status = run_with(
            "Client.exe",
            false,
            || {
                Ok(normalize_radeon_devices(
                    &json!({"gpus":[{"name":"AMD Radeon"}]}),
                ))
            },
            |_, _| {
                calls.set(calls.get() + 1);
                Err("호출되면 안 됩니다".into())
            },
        )
        .unwrap();

        assert_eq!(status["writeProtected"], true);
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn amd_apply_is_rejected_before_any_driver_write() {
        let calls = Cell::new(0);
        let error = run_with(
            "Client.exe",
            true,
            || {
                Ok(normalize_radeon_devices(
                    &json!({"gpus":[{"name":"AMD Radeon"}]}),
                ))
            },
            |_, _| {
                calls.set(calls.get() + 1);
                Err("호출되면 안 됩니다".into())
            },
        )
        .unwrap_err();

        assert!(error.contains("프로필을 보호"));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn amd_detection_failure_does_not_block_nvidia_only_systems() {
        let status = run_with(
            "Client.exe",
            false,
            || Err("WMI를 사용할 수 없습니다".into()),
            |_, _| {
                Ok(json!({
                    "supported": true,
                    "nvidia": true,
                    "gpus": [{"name":"NVIDIA GPU"}],
                    "goals": {"support": {}}
                }))
            },
        )
        .unwrap();

        assert_eq!(status["vendor"], "nvidia");
        assert_eq!(status["detected"], true);
    }
}
