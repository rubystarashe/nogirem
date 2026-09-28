use crate::{Result, http, storage};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path},
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_ARCHIVE: usize = 64 * 1024 * 1024;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
fn re(pattern: &str, value: &str) -> bool {
    regex::Regex::new(pattern).unwrap().is_match(value)
}
fn version(value: &str) -> Result<&str> {
    if re(r"^v\d+(?:\.\d+){1,3}$", value) {
        Ok(value)
    } else {
        Err("DXVK 릴리즈 버전 형식이 올바르지 않습니다".into())
    }
}
fn leaf(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '\\', ':'])
        && Path::new(value)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
}
pub fn parse_release(raw: &Value) -> Result<Value> {
    if raw["draft"] == true || raw["prerelease"] == true {
        return Err("DXVK 정식 릴리즈가 아닙니다".into());
    }
    let v = version(raw["tag_name"].as_str().unwrap_or(""))?;
    let asset = raw["assets"]
        .as_array()
        .and_then(|a| {
            a.iter().find(|a| {
                a["state"] == "uploaded"
                    && re(r"^dxvk-[\d.]+\.tar\.gz$", a["name"].as_str().unwrap_or(""))
            })
        })
        .ok_or("DXVK 릴리즈 압축 파일을 찾지 못했습니다")?;
    let url = asset["browser_download_url"]
        .as_str()
        .filter(|s| s.starts_with("https://github.com/doitsujin/dxvk/releases/download/"))
        .ok_or("DXVK 다운로드 주소가 올바르지 않습니다")?;
    let digest = asset["digest"].as_str().unwrap_or("");
    if !re(r"(?i)^sha256:[a-f0-9]{64}$", digest) {
        return Err("DXVK 릴리즈 SHA-256 정보가 없습니다".into());
    }
    Ok(
        json!({"version":v,"releaseUrl":raw["html_url"],"downloadUrl":url,"archiveName":asset["name"],"archiveSize":asset["size"],"archiveSha256":digest[7..].to_lowercase(),"publishedAt":raw["published_at"]}),
    )
}
pub fn releases() -> Result<Value> {
    let raw = http::json("https://api.github.com/repos/doitsujin/dxvk/releases?per_page=100")?;
    let list = raw
        .as_array()
        .ok_or("DXVK 릴리즈 목록 형식이 올바르지 않습니다")?
        .iter()
        .filter_map(|v| parse_release(v).ok())
        .collect::<Vec<_>>();
    if list.is_empty() {
        return Err("SHA-256 검증 가능한 DXVK 정식 릴리즈가 없습니다".into());
    }
    Ok(json!(list))
}
pub fn extract_archive(archive: &[u8]) -> Result<Vec<u8>> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(archive)
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut tar)
        .map_err(|_| "DXVK 압축 파일 해제에 실패했습니다")?;
    if tar.len() > 256 * 1024 * 1024 {
        return Err("DXVK 압축 파일 해제 한도 초과".into());
    }
    fn text(b: &[u8]) -> String {
        String::from_utf8_lossy(&b[..b.iter().position(|b| *b == 0).unwrap_or(b.len())])
            .into_owned()
    }
    let mut at = 0usize;
    while at + 512 <= tar.len() {
        let header = &tar[at..at + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name = text(&header[..100]);
        let prefix = text(&header[345..500]);
        let entry = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let size = text(&header[124..136]);
        if !re("^[0-7]+$", size.trim()) {
            return Err("DXVK 압축 파일의 TAR 크기 정보가 올바르지 않습니다".into());
        }
        let size = usize::from_str_radix(size.trim(), 8).map_err(|e| e.to_string())?;
        let start = at + 512;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= tar.len())
            .ok_or("DXVK 압축 파일의 TAR 항목 범위가 올바르지 않습니다")?;
        if matches!(header[156], 0 | 48) && re(r"(?i)^[^/]+/x64/d3d9\.dll$", &entry) {
            return Ok(tar[start..end].to_vec());
        }
        at = start
            .checked_add(size.div_ceil(512) * 512)
            .ok_or("TAR 크기 초과")?;
    }
    Err("DXVK 압축 파일에서 x64/d3d9.dll을 찾지 못했습니다".into())
}
pub fn validate_dll(dll: &[u8]) -> Result<()> {
    if dll.len() < 256 || &dll[..2] != b"MZ" {
        return Err("추출한 DXVK 파일이 Windows DLL 형식이 아닙니다".into());
    }
    let pe = u32::from_le_bytes(dll[0x3c..0x40].try_into().unwrap()) as usize;
    if pe.checked_add(6).is_none_or(|n| n > dll.len())
        || &dll[pe..pe + 4] != b"PE\0\0"
        || u16::from_le_bytes([dll[pe + 4], dll[pe + 5]]) != 0x8664
    {
        return Err("추출한 DXVK 파일이 x64 DLL 형식이 아닙니다".into());
    }
    Ok(())
}
pub fn installed(dir: &Path) -> Result<Value> {
    let current = storage::read_json(&dir.join("current.json"))?.unwrap_or(Value::Null);
    let Some(file) = current["fileName"].as_str().filter(|s| leaf(s)) else {
        return Ok(json!({"installed":false,"current":null,"integrity":null}));
    };
    match fs::read(dir.join(file)) {
        Ok(dll) => {
            let hash = http::sha256(&dll);
            Ok(
                json!({"installed":true,"integrity":current["sha256"]==hash,"actualSha256":hash,"current":current}),
            )
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(json!({"installed":false,"current":current,"integrity":false}))
        }
        Err(e) => Err(e.to_string()),
    }
}
pub fn deployment(installed: &Value, target: &Path) -> Result<Value> {
    match fs::read(target) {
        Ok(dll) => Ok(
            json!({"exists":true,"matchesCurrent":installed["installed"]==true&&installed["integrity"]==true&&installed["current"]["sha256"]==http::sha256(&dll)}),
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(json!({"exists":false,"matchesCurrent":false}))
        }
        Err(e) => Err(e.to_string()),
    }
}
fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("파일 디렉터리가 없습니다")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(
        ".dxvk-{}-{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn apply(dir: &Path, target: &Path) -> Result<Value> {
    let current = installed(dir)?;
    if current["installed"] != true || current["integrity"] != true {
        return Err("게임에 적용할 검증된 DXVK 파일이 없습니다".into());
    }
    let bytes = fs::read(dir.join(current["current"]["fileName"].as_str().unwrap()))
        .map_err(|e| e.to_string())?;
    if http::sha256(&bytes) != current["current"]["sha256"] {
        return Err("게임 폴더 적용 전 DXVK 무결성 검증에 실패했습니다".into());
    }
    atomic_bytes(target, &bytes)?;
    let result = deployment(&current, target)?;
    if result["matchesCurrent"] != true {
        return Err("게임 폴더의 d3d9_dxvk.dll 적용 검증에 실패했습니다".into());
    }
    Ok(result)
}
pub fn install(dir: &Path, v: &str, cached: Option<&Value>) -> Result<Value> {
    let v = version(v)?;
    let releases = if let Some(c) = cached {
        c.clone()
    } else {
        releases()?
    };
    let release = releases
        .as_array()
        .and_then(|a| a.iter().find(|r| r["version"] == v))
        .ok_or("선택한 DXVK 정식 릴리즈를 찾지 못했습니다")?;
    let old = installed(dir)?;
    if old["installed"] == true
        && old["integrity"] == true
        && old["current"]["version"] == v
        && old["current"]["archiveSha256"] == release["archiveSha256"]
    {
        return Ok(json!({"release":release,"installed":old,"updated":false}));
    }
    let url = release["downloadUrl"]
        .as_str()
        .filter(|s| s.starts_with("https://github.com/doitsujin/dxvk/releases/download/"))
        .ok_or("DXVK 다운로드 주소가 올바르지 않습니다")?;
    let archive = http::bytes(url, MAX_ARCHIVE, 120000, &[])?;
    if archive.is_empty() {
        return Err("DXVK 압축 파일 크기가 올바르지 않습니다".into());
    }
    if http::sha256(&archive) != release["archiveSha256"] {
        return Err("DXVK 압축 파일 SHA-256 검증에 실패했습니다".into());
    }
    let dll = extract_archive(&archive)?;
    validate_dll(&dll)?;
    let filename = format!("dxvk-{v}-d3d9.dll");
    atomic_bytes(&dir.join(&filename), &dll)?;
    let current = json!({"version":v,"fileName":filename,"sha256":http::sha256(&dll),"archiveSha256":release["archiveSha256"],"downloadUrl":url,"releaseUrl":release["releaseUrl"],"publishedAt":release["publishedAt"],"installedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true)});
    storage::write_json(&dir.join("current.json"), &current)?;
    if let Some(previous) = old["current"]["fileName"]
        .as_str()
        .filter(|f| *f != filename && leaf(f) && re(r"(?i)^dxvk-v[\d.]+-d3d9\.dll$", f))
    {
        match fs::remove_file(dir.join(previous)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(json!({"release":release,"installed":installed(dir)?,"updated":true}))
}
pub fn compatibility(v: &str, gpu: &Value) -> Value {
    let major = regex::Regex::new(r"(?i)^v?(\d+)")
        .unwrap()
        .captures(v)
        .and_then(|c| c[1].parse::<u32>().ok())
        .unwrap_or(0);
    if major < 3 {
        return json!({"compatible":true,"checked":true,"reason":null});
    }
    let nv = gpu["gpuDevice"].as_array().and_then(|a| {
        a.iter()
            .filter(|g| g["vendorId"] == 0x10de)
            .max_by_key(|g| g["gpuPreference"].as_i64().unwrap_or(0))
    });
    let driver = nv.and_then(|g| g["driverVersion"].as_str()).and_then(|s| {
        let p = s
            .split('.')
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .ok()?;
        if p.len() != 4 {
            return None;
        }
        format!("{}{:04}", p[2] % 10, p[3])
            .parse::<f64>()
            .ok()
            .map(|x| x / 100.0)
    });
    let Some(driver) = driver else {
        return json!({"compatible":true,"checked":false,"reason":null});
    };
    json!({"compatible":driver>=575.51,"checked":true,"driverVersion":format!("{driver:.2}"),"minimumDriverVersion":"575.51","reason":if driver<575.51{json!(format!("NVIDIA {driver:.2} 드라이버는 DXVK 3.x 요구사항을 충족하지 않습니다"))}else{Value::Null}})
}
pub fn dispatch(action: &str, p: &Value) -> Result<Value> {
    let dir = Path::new(p["directory"].as_str().unwrap_or(""));
    let target = Path::new(p["target"].as_str().unwrap_or(""));
    match action {
        "dxvk-releases" => releases(),
        "dxvk-installed" => installed(dir),
        "dxvk-deployment" => deployment(&p["installed"], target),
        "dxvk-apply" => apply(dir, target),
        "dxvk-install" => install(dir, p["version"].as_str().unwrap_or(""), p.get("releases")),
        "dxvk-compatibility" => Ok(compatibility(
            p["version"].as_str().unwrap_or(""),
            &p["gpuInfo"],
        )),
        _ => Err(format!("Unknown DXVK operation: {action}")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_pe_and_versions() {
        assert!(validate_dll(&[0; 300]).is_err());
        assert!(version("v../bad").is_err());
        assert!(!leaf("../d3d9.dll"));
        assert!(!leaf(r"C:\bad.dll"));
    }
    #[test]
    fn driver_minimum() {
        assert_eq!(
            compatibility(
                "v3.0",
                &json!({"gpuDevice":[{"vendorId":0x10de,"driverVersion":"32.0.15.6094"}]})
            )["compatible"],
            false
        );
        assert_eq!(compatibility("v2.7", &Value::Null)["checked"], true);
    }
    #[test]
    fn corrupt_and_truncated_archives_fail() {
        assert!(extract_archive(&[0; 100]).is_err());
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut h = [0u8; 512];
        h[0] = b'x';
        h[124..135].copy_from_slice(b"00000010000");
        gz.write_all(&h).unwrap();
        assert!(extract_archive(&gz.finish().unwrap()).is_err());
    }
}
