use crate::{
    Result, dxvk, powershell, storage,
    workers::{Environment, read},
};
use serde_json::{Value, json};
use std::path::PathBuf;
const TTL: u64 = 360000;
pub struct Manager {
    env: Environment,
    releases: Vec<Value>,
    checked: u64,
    release_error: Option<String>,
    pub runtime: Value,
    security: Option<Value>,
}
pub fn valid_release(v: &Value) -> bool {
    let check = |p: &str, k: &str| {
        regex::Regex::new(p)
            .unwrap()
            .is_match(v[k].as_str().unwrap_or(""))
    };
    check(r"^v\d+(?:\.\d+){1,3}$", "version")
        && check(r"(?i)^[a-f0-9]{64}$", "archiveSha256")
        && check(r"(?i)^dxvk-\d+(?:\.\d+){1,3}\.tar\.gz$", "archiveName")
        && v["downloadUrl"]
            .as_str()
            .is_some_and(|s| s.starts_with("https://github.com/doitsujin/dxvk/releases/download/"))
}
fn gpu_info() -> Value {
    powershell::json(r#"
$devices = @(Get-CimInstance Win32_VideoController | ForEach-Object {
  $vendor = 0
  if ($_.PNPDeviceID -match 'VEN_([0-9A-Fa-f]{4})') { $vendor = [Convert]::ToInt32($Matches[1], 16) }
  [ordered]@{ vendorId = $vendor; deviceString = $_.Name; driverVersion = $_.DriverVersion }
})
@{ gpuDevice = $devices } | ConvertTo-Json -Depth 4 -Compress
"#,15000).unwrap_or(Value::Null)
}
impl Manager {
    pub fn new(env: Environment) -> Self {
        let mut manager = Self {
            env,
            releases: vec![],
            checked: 0,
            release_error: None,
            runtime: json!({"state":"checking","latestVersion":null,"error":null}),
            security: None,
        };
        let cache = read(&manager.directory().join("releases.json"));
        if let Some(at) = cache["checkedAt"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        {
            manager.checked = at.timestamp_millis().max(0) as u64;
            manager.releases = cache["releases"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|v| valid_release(v))
                .cloned()
                .collect();
            manager.release_error = cache["error"].as_str().map(str::to_owned);
        }
        manager
    }
    pub fn directory(&self) -> PathBuf {
        self.env.user.join("vulkan")
    }
    fn target(&self) -> PathBuf {
        PathBuf::from(self.env.game()).with_file_name("d3d9_dxvk.dll")
    }
    fn releases(&mut self) -> Result<Vec<Value>> {
        if self.checked == 0 || crate::now_ms().saturating_sub(self.checked) >= TTL {
            match dxvk::releases() {
                Ok(v) => {
                    self.releases = v.as_array().cloned().unwrap_or_default();
                    self.release_error = None;
                }
                Err(e) => self.release_error = Some(e),
            }
            self.checked = crate::now_ms();
            let _ = storage::write_json(
                &self.directory().join("releases.json"),
                &json!({"checkedAt":crate::app_services::iso(),"releases":self.releases,"error":self.release_error}),
            );
        }
        if self.releases.is_empty() {
            Err(self
                .release_error
                .clone()
                .unwrap_or_else(|| "DXVK 릴리즈 목록을 확인할 수 없습니다".into()))
        } else {
            Ok(self.releases.clone())
        }
    }
    fn evaluate(&self, latest: Option<&Value>, error: Option<&str>) -> Result<Value> {
        let installed = dxvk::installed(&self.directory())?;
        let deployment = dxvk::deployment(&installed, &self.target())?;
        let compatibility = dxvk::compatibility(
            installed["current"]["version"].as_str().unwrap_or(""),
            &gpu_info(),
        );
        let applied = installed["installed"] == true
            && installed["integrity"] == true
            && deployment["matchesCurrent"] == true;
        let state = if applied && compatibility["compatible"] == false {
            "incompatible"
        } else if let Some(latest) = latest {
            if applied
                && installed["current"]["version"] == latest["version"]
                && installed["current"]["archiveSha256"] == latest["archiveSha256"]
            {
                "latest"
            } else {
                "update-required"
            }
        } else if applied {
            "applied-unverified"
        } else {
            "unavailable"
        };
        Ok(
            json!({"state":state,"latestVersion":latest.map(|v|&v["version"]).unwrap_or(&installed["current"]["version"]),"compatibility":compatibility,"error":error.map(|e|json!({"message":e}))}),
        )
    }
    pub fn load_local(&mut self) -> Result<Value> {
        let latest = read(&self.directory().join("latest.json"));
        let valid = regex::Regex::new(r"^v\d+(?:\.\d+){1,3}$")
            .unwrap()
            .is_match(latest["version"].as_str().unwrap_or(""))
            && regex::Regex::new(r"(?i)^[a-f0-9]{64}$")
                .unwrap()
                .is_match(latest["archiveSha256"].as_str().unwrap_or(""));
        self.runtime = self.evaluate(valid.then_some(&latest), None)?;
        Ok(self.runtime.clone())
    }
    pub fn refresh(&mut self) -> Result<Value> {
        let previous = self.runtime.clone();
        self.runtime = match self.releases() {
            Ok(releases) => {
                let latest = &releases[0];
                let status = self.evaluate(Some(latest), None)?;
                let _ = storage::write_json(
                    &self.directory().join("latest.json"),
                    &json!({"version":latest["version"],"archiveSha256":latest["archiveSha256"],"checkedAt":crate::app_services::iso()}),
                );
                status
            }
            Err(e) => {
                let mut local = self.evaluate(None, Some(&e))?;
                let known = matches!(
                    previous["state"].as_str(),
                    Some("latest" | "update-required")
                );
                if local["state"] == "applied-unverified" {
                    if known && previous["latestVersion"] == local["latestVersion"] {
                        local["state"] = json!("latest");
                    }
                    local
                } else if known {
                    let mut old = previous;
                    old["error"] = json!({"message":e});
                    old
                } else {
                    local
                }
            }
        };
        Ok(self.runtime.clone())
    }
    fn security(&mut self) -> Value {
        if let Some(v) = &self.security {
            return v.clone();
        }
        let v=powershell::json(r#"
$value = (Get-ItemProperty -LiteralPath 'Registry::HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\CI\Policy' -Name 'VerifiedAndReputablePolicyState' -ErrorAction SilentlyContinue).VerifiedAndReputablePolicyState
if ($null -eq $value) { $value = -1 }
$mode = switch ([int]$value) { 0 { 'off' } 1 { 'enforced' } 2 { 'evaluation' } default { 'unknown' } }
[ordered]@{ value = [int]$value; mode = $mode; blocksUnsignedDxvk = ([int]$value -eq 1) } | ConvertTo-Json -Compress
"#,5000).unwrap_or_else(|e|json!({"value":null,"mode":"unknown","blocksUnsignedDxvk":false,"error":{"message":e}}));
        self.security = Some(v.clone());
        v
    }
    pub fn status(&mut self, check_latest: bool) -> Result<Value> {
        let installed = dxvk::installed(&self.directory())?;
        let deployment = dxvk::deployment(&installed, &self.target())?;
        let security = self.security();
        let mut error = Value::Null;
        if check_latest {
            if let Err(e) = self.releases() {
                error = json!({"message":e});
            }
        }
        let latest = if error.is_null() {
            self.releases.first().cloned().unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        let mut releases = self.releases.clone();
        if releases.is_empty()
            && installed["installed"] == true
            && installed["integrity"] == true
            && installed["current"]["version"].is_string()
        {
            let mut current = installed["current"].clone();
            current["localOnly"] = json!(true);
            releases.push(current);
        }
        let gpu = gpu_info();
        for release in &mut releases {
            release["compatibility"] =
                dxvk::compatibility(release["version"].as_str().unwrap_or(""), &gpu);
        }
        let recommended = releases
            .iter()
            .find(|r| r["compatibility"]["compatible"] != false)
            .cloned()
            .unwrap_or(Value::Null);
        let desired = if recommended.is_null() {
            &latest
        } else {
            &recommended
        };
        let update = if desired.is_null() {
            Value::Null
        } else {
            json!(
                installed["installed"] != true
                    || installed["integrity"] != true
                    || deployment["matchesCurrent"] != true
                    || installed["current"]["version"] != desired["version"]
                    || installed["current"]["archiveSha256"] != desired["archiveSha256"]
            )
        };
        Ok(
            json!({"installedCompatibility":dxvk::compatibility(installed["current"]["version"].as_str().unwrap_or(""),&gpu),"installed":installed,"deployment":deployment,"securityPolicy":security,"latest":latest,"recommended":recommended,"releases":releases,"releaseCheckError":error,"updateAvailable":update,"storagePath":self.directory()}),
        )
    }
    pub fn install(&mut self, version: &str) -> Result<Value> {
        if crate::process::game_active(&read(&self.env.root.join("config.json")))? {
            return Err("마비노기가 실행 중일 때에는 DXVK를 교체할 수 없습니다".into());
        }
        let compatibility = dxvk::compatibility(version, &gpu_info());
        if compatibility["compatible"] == false {
            return Err(compatibility["reason"]
                .as_str()
                .unwrap_or("지원하지 않는 드라이버")
                .into());
        }
        let installed = dxvk::installed(&self.directory())?;
        let mut result = if installed["installed"] == true
            && installed["integrity"] == true
            && installed["current"]["version"] == version
        {
            json!({"installed":installed,"updated":false})
        } else {
            let releases = json!(self.releases()?);
            dxvk::install(&self.directory(), version, Some(&releases))?
        };
        // The game can start while a download is in flight. Recheck before replacing its DLL.
        if crate::process::game_active(&read(&self.env.root.join("config.json")))? {
            return Err("마비노기가 실행 중일 때에는 DXVK를 교체할 수 없습니다".into());
        }
        result["deployment"] = dxvk::apply(&self.directory(), &self.target())?;
        result["storagePath"] = json!(self.directory());
        result["runtimeStatus"] = self.refresh()?;
        Ok(result)
    }
}
