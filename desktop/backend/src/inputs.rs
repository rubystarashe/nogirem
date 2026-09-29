use crate::{
    Result, http, settings, storage, topology,
    workers::{Environment, ManagedProcess, read, remove},
};
use serde_json::{Value, json};
use std::{fs, path::Path};
const VERSION: &str = "0.1.7";
fn input_normalize(v: &Value) -> Value {
    let scale = v["cursorScalePercent"]
        .as_i64()
        .or_else(|| {
            v["cursorScalePercent"]
                .as_str()
                .and_then(|s| s.parse().ok())
        })
        .filter(|n| (25..=800).contains(n) && n % 25 == 0)
        .unwrap_or(100);
    let modifier = v["cursorWheelModifier"]
        .as_str()
        .filter(|s| ["disabled", "control", "alt"].contains(s))
        .unwrap_or("disabled");
    json!({"enabled":v["enabled"]==true,"cursorScalePercent":scale,"cursorWheelModifier":modifier})
}
fn required(v: &Value) -> bool {
    v["enabled"] == true || v["cursorScalePercent"] != 100 || v["cursorWheelModifier"] != "disabled"
}
fn validate_pe(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 256 || bytes.len() > 2 * 1024 * 1024 {
        return Err("터보 키 실행 파일의 크기가 올바르지 않습니다".into());
    }
    crate::dxvk::validate_dll(bytes)?;
    if u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) < 0x40 {
        return Err("터보 키 실행 파일의 PE 위치가 올바르지 않습니다".into());
    }
    Ok(())
}
pub struct Inputs {
    env: Environment,
    input: ManagedProcess,
    turbo: ManagedProcess,
}
impl Inputs {
    pub fn new(env: Environment) -> Self {
        Self {
            input: ManagedProcess::new(env.user.join("input-guard")),
            turbo: ManagedProcess::new(env.user.join("turbo-key")),
            env,
        }
    }
    pub fn input_status(&mut self) -> Result<Value> {
        let file = self.input.directory.join("settings.json");
        let mut setting = input_normalize(&read(&file));
        let status = self.input.status();
        let running = self.input.running() && status["running"] == true;
        let runtime_scale = status["cursorScalePercent"]
            .as_i64()
            .filter(|n| (25..=800).contains(n) && n % 25 == 0);
        if running && runtime_scale.is_some_and(|n| setting["cursorScalePercent"] != n) {
            setting["cursorScalePercent"] = json!(runtime_scale.unwrap());
            self.save(&file, &setting)?
        }
        let reason = status["error"].as_str().map(str::to_owned).or_else(|| {
            (required(&setting) && !running)
                .then(|| "마비노기 입력 기능 helper가 실행 중이 아닙니다".into())
        });
        setting["running"] = json!(running);
        setting["cursorActive"] = json!(status["cursorActive"] == true);
        setting["gameOnly"] = json!(true);
        setting["reason"] = json!(reason);
        Ok(setting)
    }
    fn save(&self, path: &Path, value: &Value) -> Result<()> {
        let mut value = value.clone();
        value["updatedAt"] = json!(crate::now_ms());
        storage::write_json(path, &value)
    }
    fn launch_input(&mut self, setting: &Value) -> Result<()> {
        let game = self.env.game();
        if !self.env.valid_game(&game) {
            return Err("마비노기 실행 경로를 확인하지 못했습니다".into());
        }
        let old = self.input.status();
        let mut args = vec![
            format!("--game-path={game}"),
            format!(
                "--alt-enter-enabled={}",
                if setting["enabled"] == true { 1 } else { 0 }
            ),
            format!("--cursor-scale-percent={}", setting["cursorScalePercent"]),
            format!(
                "--cursor-wheel-modifier={}",
                setting["cursorWheelModifier"].as_str().unwrap()
            ),
        ];
        if old["cursorActive"] == true {
            if let Some(n) = old["originalCursorBaseSize"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= 256)
            {
                args.push(format!("--restore-cursor-base-size={n}"))
            }
        }
        self.input.start(
            &self
                .env
                .root
                .join("native/input-guard-helper/bin/input-guard-helper.exe"),
            args,
            3000,
            None,
        )?;
        Ok(())
    }
    pub fn set_input(&mut self, patch: &Value) -> Result<Value> {
        let file = self.input.directory.join("settings.json");
        let old = input_normalize(&read(&file));
        let mut next = old.clone();
        if patch.is_boolean() {
            next["enabled"] = patch.clone()
        } else if let Some(fields) = patch.as_object() {
            for (k, v) in fields {
                if k == "enabled" && !v.is_boolean() {
                    return Err("입력 기능 활성화 값 오류".into());
                }
                if k == "cursorScalePercent"
                    && !v
                        .as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                        .is_some_and(|n| (25..=800).contains(&n) && n % 25 == 0)
                {
                    return Err("커서 크기 값 오류".into());
                }
                if k == "cursorWheelModifier"
                    && !matches!(v.as_str(), Some("disabled" | "control" | "alt"))
                {
                    return Err("커서 휠 조합 값 오류".into());
                }
                next[k] = v.clone();
            }
        } else {
            return Err("마비노기 입력 기능 설정 값이 올바르지 않습니다".into());
        }
        next = input_normalize(&next);
        self.input.stop()?;
        let result = (|| {
            self.save(&file, &next)?;
            if required(&next) {
                self.launch_input(&next)?
            }
            Ok::<_, String>(())
        })();
        if let Err(e) = result {
            let _ = self.input.stop();
            let _ = self.save(&file, &old);
            if required(&old) {
                let _ = self.launch_input(&old);
            }
            return Err(e);
        }
        self.input_status()
    }
    pub fn installation(&self) -> Result<Value> {
        let dir = &self.turbo.directory;
        let local = self
            .env
            .root
            .join("native/turbo-key/bin/turbo-key-helper.exe");
        let path = if self.env.packaged {
            dir.join("bin/turbo-key-helper.exe")
        } else {
            local
        };
        let manifest = read(&dir.join("current.json"));
        let mut result = json!({"installed":false,"updateRequired":false,"executablePath":path.to_string_lossy(),"helperVersion":VERSION,"installedVersion":manifest["helperVersion"],"reason":null});
        if self.env.packaged
            && !manifest["sha256"]
                .as_str()
                .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Ok(result);
        }
        let validate: Result<String> = (|| {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            validate_pe(&bytes)?;
            let hash = http::sha256(&bytes);
            if self.env.packaged && manifest["sha256"].as_str().unwrap().to_lowercase() != hash {
                return Err("터보 키 실행 파일의 무결성 검증에 실패했습니다".into());
            }
            Ok(hash)
        })();
        match validate {
            Ok(hash) => {
                let update = self.env.packaged
                    && (manifest["helperVersion"] != VERSION || manifest["protocolVersion"] != 1);
                result["installed"] = json!(!update);
                result["updateRequired"] = json!(update);
                result["protocolVersion"] = json!(1);
                result["sha256"] = json!(hash);
                result["installedVersion"] = if self.env.packaged {
                    manifest["helperVersion"].clone()
                } else {
                    json!(VERSION)
                };
                for key in ["installedAt", "acceptedAt", "source"] {
                    if self.env.packaged {
                        result[key] = manifest[key].clone();
                    }
                }
                if !self.env.packaged {
                    result["source"] = json!("local-development-build")
                }
                if update {
                    result["reason"] = json!("터보 키 업데이트가 필요합니다")
                }
            }
            Err(e) => {
                if path.exists() {
                    result["reason"] = json!(e)
                } else if !self.env.packaged {
                    result["reason"] = json!("로컬 터보 키 helper를 먼저 빌드하세요")
                }
            }
        }
        Ok(result)
    }
    pub fn turbo_status(&mut self) -> Result<Value> {
        let mut setting = settings::turbo(&read(&self.turbo.directory.join("settings.json")));
        let status = self.turbo.status();
        let installation = self.installation()?;
        let fresh =
            crate::now_ms().saturating_sub(status["updatedAt"].as_u64().unwrap_or(0)) < 5000;
        let enabled = setting["enabled"] == true && installation["installed"] == true;
        let running = self.turbo.running() && status["running"] == true && fresh;
        let reason = installation["reason"]
            .as_str()
            .or_else(|| {
                if fresh {
                    status["error"].as_str()
                } else {
                    None
                }
            })
            .map(str::to_owned)
            .or_else(|| {
                (enabled && !running).then(|| "터보 키 프로세스가 실행 중이 아닙니다".into())
            });
        for k in ["installed", "updateRequired", "helperVersion"] {
            setting[k] = installation[k].clone()
        }
        setting["enabled"] = json!(enabled);
        setting["running"] = json!(running);
        setting["repeatHz"] =
            json!((1000. / setting["intervalMs"].as_f64().unwrap()).round() as u64);
        setting["gameOnly"] = json!(true);
        setting["reason"] = json!(reason);
        Ok(setting)
    }
    fn launch_turbo(&mut self, s: &Value) -> Result<()> {
        let installation = self.installation()?;
        if installation["installed"] != true {
            return Err(installation["reason"]
                .as_str()
                .unwrap_or("터보 키를 먼저 다운로드하세요")
                .into());
        }
        let allocation = topology::resolve(self.env.core_count())?;
        let mask = if allocation.alternate_game_mask != 0 {
            allocation.alternate_game_mask
        } else {
            allocation.background_mask
        };
        let args = vec![
            format!("--affinity-mask=0x{mask:x}"),
            format!(
                "--keys={}",
                settings::turbo_keys(&s["keys"])
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            format!("--interval-ms={}", s["intervalMs"]),
            format!("--ignore-initial-delay={}", s["ignoreInitialDelay"]),
        ];
        self.turbo.start(
            Path::new(installation["executablePath"].as_str().unwrap()),
            args,
            3000,
            None,
        )?;
        Ok(())
    }
    pub fn set_turbo(&mut self, v: &Value) -> Result<Value> {
        if !v["enabled"].is_boolean()
            || !v["keys"].is_array()
            || !v["ignoreInitialDelay"].is_boolean()
            || !v["intervalMs"]
                .as_u64()
                .is_some_and(|n| [1, 3, 5, 10, 20, 30].contains(&n))
        {
            return Err("터보 키 설정 값이 올바르지 않습니다".into());
        }
        let value = settings::turbo(v);
        self.turbo.stop()?;
        if value["enabled"] == true {
            self.launch_turbo(&value)?
        }
        if let Err(e) = self.save(&self.turbo.directory.join("settings.json"), &value) {
            let _ = self.turbo.stop();
            return Err(e);
        }
        self.turbo_status()
    }
    fn install_binary(&mut self, version: &str, accepted: Value) -> Result<()> {
        self.turbo.stop()?;
        if self.env.packaged {
            let release = http::json(&format!(
                "https://api.github.com/repos/rubystarashe/nogirem/releases/tags/v{version}"
            ))?;
            if release["draft"] == true || release["prerelease"] == true {
                return Err("정식 Release의 터보 키 실행 파일만 다운로드할 수 있습니다".into());
            }
            let asset = release["assets"]
                .as_array()
                .and_then(|a| {
                    a.iter().find(|a| {
                        a["name"] == format!("turbo-key-helper-win32-x64-v{VERSION}.exe")
                            && a["state"] == "uploaded"
                    })
                })
                .ok_or("현재 버전에 맞는 터보 키 다운로드 파일이 없습니다")?;
            if !asset["size"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 2 * 1024 * 1024)
            {
                return Err("터보 키 다운로드 파일의 크기가 올바르지 않습니다".into());
            }
            let digest = asset["digest"]
                .as_str()
                .and_then(|s| s.strip_prefix("sha256:"))
                .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or("터보 키 다운로드 파일의 SHA-256 정보가 없습니다")?;
            let url = asset["browser_download_url"]
                .as_str()
                .filter(|s| s.starts_with("https://github.com/"))
                .ok_or("허용되지 않은 터보 키 다운로드 주소입니다")?;
            let bytes = http::bytes(
                url,
                2 * 1024 * 1024,
                15000,
                &[("Accept", "application/octet-stream")],
            )?;
            if http::sha256(&bytes) != digest.to_lowercase() {
                return Err("다운로드한 터보 키 실행 파일의 SHA-256이 일치하지 않습니다".into());
            }
            validate_pe(&bytes)?;
            let bin = self.turbo.directory.join("bin");
            fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
            let temp = bin.join(format!(".turbo-{}.tmp", std::process::id()));
            fs::write(&temp, &bytes).map_err(|e| e.to_string())?;
            fs::rename(&temp, bin.join("turbo-key-helper.exe")).map_err(|e| e.to_string())?;
            storage::write_json(
                &self.turbo.directory.join("current.json"),
                &json!({"helperVersion":VERSION,"protocolVersion":1,"sha256":http::sha256(&bytes),"source":release["html_url"],"acceptedAt":accepted,"installedAt":crate::now_ms()}),
            )?;
        }
        if self.installation()?["installed"] != true {
            return Err("터보 키 설치 상태를 확인하지 못했습니다".into());
        }
        Ok(())
    }
    pub fn install(&mut self, version: &str) -> Result<Value> {
        self.install_binary(version, json!(crate::now_ms()))?;
        let mut s = settings::turbo(&read(&self.turbo.directory.join("settings.json")));
        s["enabled"] = json!(false);
        self.save(&self.turbo.directory.join("settings.json"), &s)?;
        self.turbo_status()
    }
    pub fn uninstall(&mut self) -> Result<Value> {
        self.turbo.stop()?;
        let mut s = settings::turbo(&read(&self.turbo.directory.join("settings.json")));
        s["enabled"] = json!(false);
        self.save(&self.turbo.directory.join("settings.json"), &s)?;
        remove(&self.turbo.directory.join("bin/turbo-key-helper.exe"))?;
        remove(&self.turbo.directory.join("current.json"))?;
        self.turbo_status()
    }
    pub fn start_saved(&mut self) -> Result<()> {
        let input = input_normalize(&read(&self.input.directory.join("settings.json")));
        let mut errors = vec![];
        if required(&input) {
            if let Err(e) = self.launch_input(&input) {
                errors.push(e)
            }
        }
        let turbo = settings::turbo(&read(&self.turbo.directory.join("settings.json")));
        let installed = self.installation()?;
        if self.env.packaged
            && installed["updateRequired"] == true
            && !installed["acceptedAt"].is_null()
        {
            if let Err(e) =
                self.install_binary(env!("CARGO_PKG_VERSION"), installed["acceptedAt"].clone())
            {
                errors.push(e);
            }
        }
        if turbo["enabled"] == true && self.installation()?["installed"] == true {
            if let Err(e) = self.launch_turbo(&turbo) {
                errors.push(e)
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" / "))
        }
    }
    pub fn restart_input(&mut self) -> Result<()> {
        let setting = input_normalize(&read(&self.input.directory.join("settings.json")));
        self.input.stop()?;
        if required(&setting) {
            self.launch_input(&setting)?;
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        let a = self.input.stop();
        let b = self.turbo.stop();
        a.and(b)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_defaults_and_bounds() {
        assert_eq!(input_normalize(&Value::Null)["cursorScalePercent"], 100);
        assert_eq!(
            input_normalize(&json!({"cursorScalePercent":25}))["cursorScalePercent"],
            25
        );
        assert_eq!(
            input_normalize(&json!({"cursorScalePercent":24}))["cursorScalePercent"],
            100
        );
        assert_eq!(
            input_normalize(&json!({"cursorScalePercent":801}))["cursorScalePercent"],
            100
        );
        assert!(required(&input_normalize(
            &json!({"cursorScalePercent":200})
        )));
        assert!(!required(&input_normalize(&json!({}))));
    }
}
