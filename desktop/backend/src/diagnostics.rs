use crate::{Result, app_services, powershell, service::Service};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
pub fn redact(value: &str, private: &[String]) -> String {
    let mut text = value.to_owned();
    let mut paths = private.to_vec();
    for key in ["USERPROFILE", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(key) {
            paths.push(v);
        }
    }
    paths.sort_by_key(|s| std::cmp::Reverse(s.len()));
    paths.dedup();
    for path in paths.into_iter().filter(|s| !s.is_empty()) {
        for value in [path.replace('\\', "\\\\"), path] {
            let re = regex::RegexBuilder::new(&regex::escape(&value))
                .case_insensitive(true)
                .build()
                .unwrap();
            text = re.replace_all(&text, "%USER_DATA%").into_owned();
        }
    }
    for pattern in [
        r#"(?i)(?:[A-Za-z]:\\)?Users\\[^\\/\s"]+"#,
        r#"(?i)(?:[A-Za-z]:\\\\)?Users\\\\[^\\/\s"]+"#,
    ] {
        text = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&text, "%USERPROFILE%")
            .into_owned();
    }
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if !name.is_empty() {
            let re = regex::Regex::new(&format!(r"(?i)(\\\\){}(\\|\s|$)", regex::escape(&name)))
                .unwrap();
            text = re.replace_all(&text, "${1}%COMPUTERNAME%${2}").into_owned();
        }
    }
    text = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
        .unwrap()
        .replace_all(&text, |c: &regex::Captures| {
            if c[0].split('.').all(|s| s.parse::<u8>().is_ok()) {
                String::from("%IP_ADDRESS%")
            } else {
                c[0].to_owned()
            }
        })
        .into_owned();
    for (pattern, replacement) in [
        (
            r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b",
            "%EMAIL_ADDRESS%",
        ),
        (
            r#"(?i)("(?:access_?token|refresh_?token|authorization|password|cookie|secret)"\s*:\s*)"[^"]*""#,
            r#"${1}"%REDACTED%""#,
        ),
        (r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+", "Bearer %REDACTED%"),
        (r"\b(?:ghp|github_pat)_[A-Za-z0-9_]+", "%REDACTED_TOKEN%"),
        (
            r"(?i)([?&](?:token|key|secret|signature|sig)=)[^&#\s]+",
            "${1}%REDACTED%",
        ),
    ] {
        text = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&text, replacement)
            .into_owned();
    }
    text
}
struct Entry {
    name: String,
    content: String,
    original: u64,
    included: usize,
    truncated: bool,
}
fn collect(root: &Path, private: &[String]) -> Vec<Entry> {
    fn walk(
        root: &Path,
        dir: &Path,
        depth: usize,
        private: &[String],
        bytes: &mut usize,
        entries: &mut Vec<Entry>,
    ) {
        if depth > 6 || *bytes >= 25 * 1024 * 1024 {
            return;
        }
        let Ok(list) = fs::read_dir(dir) else { return };
        let mut list = list.filter_map(std::result::Result::ok).collect::<Vec<_>>();
        list.sort_by_key(|e| e.file_name());
        for entry in list {
            if *bytes >= 25 * 1024 * 1024 {
                break;
            }
            let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if meta.is_dir() {
                if ![
                    "blob_storage",
                    "cache",
                    "code cache",
                    "crashpad",
                    "dawncache",
                    "gpucache",
                    "clips",
                    "ring",
                    "bin",
                    "session storage",
                    "local storage",
                ]
                .contains(&name.as_str())
                {
                    walk(root, &entry.path(), depth + 1, private, bytes, entries);
                }
                continue;
            }
            if !meta.is_file()
                || !(name.ends_with(".log.previous")
                    || ["json", "lock", "log", "txt", "yml", "yaml"].contains(
                        &entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase()
                            .as_str(),
                    ))
            {
                continue;
            }
            let count = (meta.len() as usize)
                .min(5 * 1024 * 1024)
                .min(25 * 1024 * 1024 - *bytes);
            let read = (|| -> std::io::Result<Vec<u8>> {
                let mut file = fs::File::open(entry.path())?;
                file.seek(SeekFrom::Start(meta.len() - count as u64))?;
                let mut data = vec![];
                file.take(count as u64).read_to_end(&mut data)?;
                Ok(data)
            })();
            let Ok(data) = read else { continue };
            *bytes += data.len();
            let truncated = meta.len() > data.len() as u64;
            let decoded = if data.starts_with(&[0xff, 0xfe]) {
                String::from_utf16_lossy(
                    &data[2..]
                        .chunks_exact(2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .collect::<Vec<_>>(),
                )
            } else {
                String::from_utf8_lossy(&data).into_owned()
            };
            let text = if truncated {
                format!(
                    "[파일이 커서 최근 {}바이트만 포함됨]\n{decoded}",
                    data.len()
                )
            } else {
                decoded
            };
            if let Ok(relative) = entry.path().strip_prefix(root) {
                entries.push(Entry {
                    name: relative.to_string_lossy().replace('\\', "/"),
                    content: redact(&text, private),
                    original: meta.len(),
                    included: data.len(),
                    truncated,
                });
            }
        }
    }
    let mut entries = vec![];
    walk(root, root, 0, private, &mut 0, &mut entries);
    entries
}
pub fn bundle(
    output: &Path,
    user: &Path,
    diagnostics: &Value,
    id: &str,
    private: &[String],
) -> Result<usize> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err("진단 로그 고유값이 올바르지 않습니다".into());
    }
    let mut private = private.to_vec();
    private.push(user.to_string_lossy().into_owned());
    let entries = collect(user, &private);
    let temp = output.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let write = (|| {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut add = |name: &str, content: &str| -> Result<()> {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(content.as_bytes()).map_err(|e| e.to_string())
        };
        add(
            "diagnostics.json",
            &redact(
                &serde_json::to_string_pretty(diagnostics).unwrap(),
                &private,
            ),
        )?;
        add(
            "report.json",
            &json!({"schemaVersion":1,"reportId":id,"createdAt":diagnostics["generatedAt"]})
                .to_string(),
        )?;
        for entry in &entries {
            add(&format!("files/{}", entry.name), &entry.content)?;
        }
        add("included-files.json",&json!(entries.iter().map(|e|json!({"name":e.name,"originalBytes":e.original,"includedBytes":e.included,"truncated":e.truncated})).collect::<Vec<_>>()).to_string())?;
        zip.finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, output).map_err(|e| e.to_string())?;
        Ok(entries.len() + 3)
    })();
    if write.is_err() {
        let _ = fs::remove_file(temp);
    }
    write
}
fn diagnostic(result: Result<Value>) -> Value {
    result.unwrap_or_else(|e| json!({"error":{"message":e}}))
}
pub fn export(s: &Service) -> Result<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let version = env!("CARGO_PKG_VERSION");
    let downloads = std::env::var("NOGIREM_KNOWN_FOLDERS")
        .ok()
        .and_then(|v| serde_json::from_str::<Value>(&v).ok())
        .and_then(|v| v["downloads"].as_str().map(PathBuf::from))
        .unwrap_or_else(|| s.env.documents.clone());
    let chosen=s.rpc.request("dialog.showSaveDialog",json!({"windowId":1,"options":{"title":"진단 로그 압축파일 저장","defaultPath":downloads.join(format!("nogirem-diagnostics-{version}-{timestamp}-{id}.zip")),"buttonLabel":"진단 로그 저장","filters":[{"name":"ZIP 압축파일","extensions":["zip"]}],"properties":["createDirectory","showOverwriteConfirmation"]}}))?;
    if chosen["canceled"] == true || !chosen["filePath"].is_string() {
        return Ok(json!({"canceled":true}));
    }
    let mut output = chosen["filePath"].as_str().unwrap().to_owned();
    if !output.to_lowercase().ends_with(".zip") {
        output.push_str(".zip");
    }
    let system = diagnostic(powershell::json(
        r#"$os=Get-CimInstance Win32_OperatingSystem
$cpu=@(Get-CimInstance Win32_Processor | ForEach-Object { [ordered]@{model=$_.Name;logicalProcessors=$_.NumberOfLogicalProcessors} })
$gpu=@(Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,PNPDeviceID)
[ordered]@{platform='win32';type='Windows_NT';release=$os.Version;version=$os.Caption;architecture=$env:PROCESSOR_ARCHITECTURE;uptimeSeconds=[math]::Round(((Get-Date)-$os.LastBootUpTime).TotalSeconds);totalMemoryBytes=[long]$os.TotalVisibleMemorySize*1024;freeMemoryBytes=[long]$os.FreePhysicalMemory*1024;logicalCpuCount=[Environment]::ProcessorCount;processorModels=$cpu;locale=(Get-Culture).Name;gpu=$gpu}|ConvertTo-Json -Depth 6 -Compress"#,
        15000,
    ));
    let failures = diagnostic(powershell::json(
        r#"$events=@(Get-WinEvent -FilterHashtable @{LogName='System';Id=41,1001,6008;StartTime=(Get-Date).AddDays(-14)} -ErrorAction SilentlyContinue | Select-Object -First 30)
@($events | ForEach-Object {[ordered]@{timeCreated=if($_.TimeCreated){$_.TimeCreated.ToUniversalTime().ToString('o')}else{$null};id=$_.Id;provider=$_.ProviderName;level=$_.LevelDisplayName;message=$_.Message}}) | ConvertTo-Json -Depth 3 -Compress"#,
        15000,
    ));
    let mut system = system;
    if system.is_object() {
        system["recentWindowsFailures"] = failures;
    }
    let turbo = diagnostic(s.inputs.lock().unwrap().turbo_status());
    let input = diagnostic(s.inputs.lock().unwrap().input_status());
    let diagnostics = json!({"reportId":id,"generatedAt":app_services::iso(),"application":{"name":"마비노기 렘 부스터","version":version,"packaged":s.env.packaged,"executablePath":s.env.exe,"applicationPath":s.env.root,"startupTrayLaunch":s.startup_tray},"runtime":{"desktop":"dioxus","webview":"WebView2","backend":"rust"},"system":system,"applicationState":{"activeMabinogiExecutablePath":s.env.game(),"turboKey":turbo,"inputGuard":input,"blackbox":diagnostic(s.blackbox.lock().unwrap().status()),"startupTray":diagnostic(app_services::startup(&s.env.exe,s.env.packaged,None)),"affinity":diagnostic(s.boost.affinity_runtime()),"dxvk":s.dxvk_runtime.read().unwrap().clone(),"network":diagnostic(crate::network_manager::dispatch("status",&s.env.user.join("network/fast-ping-original.json")))}});
    let count = bundle(
        Path::new(&output),
        &s.env.user,
        &diagnostics,
        &id,
        &[
            s.env.root.to_string_lossy().into_owned(),
            s.env.exe.to_string_lossy().into_owned(),
        ],
    )?;
    app_services::own_report(&s.env.user, &id)?;
    s.rpc
        .request("shell.showItemInFolder", json!({"path":output}))?;
    Ok(json!({"canceled":false,"filePath":output,"fileCount":count,"reportId":id}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secrets_and_paths_are_redacted() {
        let source = r#"{"password":"abc","path":"C:\Users\private\file","ip":"192.168.0.1","email":"hello@example.com"} Bearer abc.123 ghp_abcdefgh"#;
        let s = redact(source, &[]);
        assert!(!s.contains("private"));
        assert!(!s.contains("192.168.0.1"));
        assert!(!s.contains("hello@example.com"));
        assert!(!s.contains("abc.123"));
        assert!(!s.contains("abcdefgh"));
        assert!(s.contains("%REDACTED%"));
    }
    #[test]
    fn exported_zip_redacts_private_data_and_excludes_recordings() {
        let root =
            std::env::temp_dir().join(format!("nogirem-diagnostics-test-{}", uuid::Uuid::new_v4()));
        let user = root.join("user");
        fs::create_dir_all(user.join("Ring")).unwrap();
        fs::write(user.join("Ring/private.json"), b"must not collect").unwrap();
        fs::write(
            user.join("events.log"),
            r#"{"password":"fixture-secret","ip":"192.168.1.7"}"#,
        )
        .unwrap();
        fs::write(user.join("video.mp4"), b"must not collect").unwrap();
        let path = root.join("report.zip");
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            bundle(
                &path,
                &user,
                &json!({"generatedAt":"fixture","home":user}),
                &id,
                &[]
            )
            .unwrap(),
            4
        );
        let mut zip = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(zip.len(), 4);
        assert!(zip.by_name("files/Ring/private.json").is_err());
        assert!(zip.by_name("files/video.mp4").is_err());
        let mut text = String::new();
        zip.by_name("files/events.log")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert!(!text.contains("fixture-secret"));
        assert!(!text.contains("192.168.1.7"));
        text.clear();
        zip.by_name("diagnostics.json")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert!(text.contains("%USER_DATA%"));
        drop(zip);
        fs::remove_dir_all(root).unwrap();
    }
}
