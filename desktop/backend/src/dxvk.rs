use crate::{Result, http, storage};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path},
};
const MAX_ARCHIVE: usize = 64 * 1024 * 1024;
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
fn file_error(action: &str, error: std::io::Error) -> String {
    match error.raw_os_error() {
        Some(2 | 3) => format!(
            "{action} 중 파일이 사라졌습니다. Windows 보안 또는 백신의 보호 기록에서 DXVK 격리 여부를 확인해 주세요 ({error})"
        ),
        Some(5) => format!(
            "{action} 접근이 거부되었습니다. Windows 보안·백신 차단, 폴더 쓰기 권한 또는 다른 프로그램의 파일 사용 여부를 확인해 주세요 ({error})"
        ),
        Some(32 | 33) => format!(
            "{action} 대상 파일이 사용 중입니다. 마비노기와 해당 폴더를 사용하는 프로그램을 종료한 뒤 다시 시도해 주세요 ({error})"
        ),
        _ => format!("{action}에 실패했습니다 ({error})"),
    }
}
fn is_reparse(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(
            metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                != 0,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(file_error("DXVK 대상 파일 정보 확인", error)),
    }
}
fn handle_replace(file: &File, target: &Path, action: &str) -> Result<()> {
    let name = target.as_os_str().encode_wide().collect::<Vec<_>>();
    let offset = std::mem::offset_of!(
        windows_sys::Win32::Storage::FileSystem::FILE_RENAME_INFO,
        FileName
    );
    let size = offset + name.len() * std::mem::size_of::<u16>();
    let mut buffer = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
    let info =
        buffer.as_mut_ptr() as *mut windows_sys::Win32::Storage::FileSystem::FILE_RENAME_INFO;
    unsafe {
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = std::ptr::null_mut();
        (*info).FileNameLength = (name.len() * std::mem::size_of::<u16>()) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
            name.len(),
        );
    }
    let renamed = unsafe {
        windows_sys::Win32::Storage::FileSystem::SetFileInformationByHandle(
            file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
            windows_sys::Win32::Storage::FileSystem::FileRenameInfo,
            buffer.as_ptr().cast(),
            size as u32,
        )
    };
    if renamed == 0 {
        Err(file_error(action, std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}
fn file_identity(file: &File, action: &str) -> Result<(u32, u64)> {
    let mut info = unsafe {
        std::mem::zeroed::<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION>()
    };
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
            file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
            &mut info,
        )
    };
    if ok == 0 {
        return Err(file_error(action, std::io::Error::last_os_error()));
    }
    Ok((
        info.dwVolumeSerialNumber,
        ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
    ))
}
fn path_identity(path: &Path, action: &str) -> Result<(u32, u64)> {
    let file = fs::OpenOptions::new()
        .read(true)
        .share_mode(
            windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
        )
        .open(path)
        .map_err(|error| file_error(action, error))?;
    file_identity(&file, action)
}
fn lock_directory(path: &Path, action: &str) -> Result<File> {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(
            windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
        )
        .custom_flags(
            windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS
                | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
        )
        .open(path)
        .map_err(|error| file_error(action, error))
}
fn directory_path_identity(path: &Path, action: &str) -> Result<(u32, u64)> {
    let directory = lock_directory(path, action)?;
    file_identity(&directory, action)
}
#[derive(Debug)]
struct PendingReplacement {
    target: std::path::PathBuf,
    backup: Option<(std::path::PathBuf, String)>,
    action: String,
}
impl PendingReplacement {
    fn commit(self) -> Result<()> {
        if let Some((backup, _)) = self.backup {
            match fs::remove_file(backup) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(file_error(
                        &format!("{} 기존 파일 backup 정리", self.action),
                        error,
                    ));
                }
            }
        }
        Ok(())
    }
    fn rollback(self, original_error: String) -> String {
        if let Some((backup, expected)) = self.backup {
            let backup_name = backup
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "DXVK backup".into());
            let restore = backup.with_file_name(format!(".dxvk-restore-{}.tmp", uuid::Uuid::new_v4()));
            let prepared: Result<File> = (|| {
                let saved =
                    fs::read(&backup).map_err(|error| file_error("DXVK backup 읽기", error))?;
                if http::sha256(&saved) != expected {
                    return Err("DXVK backup 무결성 검증에 실패했습니다".into());
                }
                let mut restore_file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .access_mode(
                        windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_READ
                            | windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_WRITE
                            | windows_sys::Win32::Storage::FileSystem::DELETE,
                    )
                    .share_mode(
                        windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                            | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                            | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
                    )
                    .open(&restore)
                    .map_err(|error| file_error("DXVK 복구 임시 파일 생성", error))?;
                restore_file
                    .write_all(&saved)
                    .map_err(|error| file_error("DXVK 복구 임시 파일 기록", error))?;
                restore_file
                    .sync_all()
                    .map_err(|error| file_error("DXVK 복구 임시 파일 동기화", error))?;
                Ok(restore_file)
            })();
            let restore_file = match prepared {
                Ok(file) => file,
                Err(restore_error) => {
                    let _ = fs::remove_file(&restore);
                    return format!(
                        "{original_error}; 기존 DXVK 파일 복구 준비에 실패했습니다. 수동 복구용 {backup_name} 파일은 보존했습니다: {restore_error}"
                    );
                }
            };
            if let Err(restore_error) = handle_replace(
                &restore_file,
                &self.target,
                &format!("{} 기존 파일 복구", self.action),
            ) {
                let _ = fs::remove_file(&restore);
                return format!(
                    "{original_error}; 기존 DXVK 파일 복구에도 실패했습니다. 수동 복구용 {backup_name} 파일은 보존했습니다: {restore_error}"
                );
            }
            return match fs::read(&self.target) {
                Ok(restored) if http::sha256(&restored) == expected => {
                    let _ = fs::remove_file(&backup);
                    original_error
                }
                Ok(_) => format!(
                    "{original_error}; 기존 DXVK 파일 복구 후 무결성 검증에 실패했습니다. 수동 복구용 {backup_name} 파일은 보존했습니다"
                ),
                Err(error) => format!(
                    "{original_error}; {}; 수동 복구용 {backup_name} 파일은 보존했습니다",
                    file_error("기존 DXVK 파일 복구 확인", error)
                ),
            };
        }
        match fs::remove_file(&self.target) {
            Ok(()) => original_error,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => original_error,
            Err(error) => format!(
                "{original_error}; {}",
                file_error("실패한 DXVK 대상 파일 정리", error)
            ),
        }
    }
}
fn atomic_bytes_guarded(
    path: &Path,
    bytes: &[u8],
    action: &str,
    before_replace: impl FnOnce() -> Result<()>,
) -> Result<PendingReplacement> {
    let parent = path.parent().ok_or("DXVK 대상 디렉터리가 없습니다")?;
    if parent.as_os_str().is_empty() {
        return Err("DXVK 대상 디렉터리가 비어 있습니다".into());
    }
    fs::create_dir_all(parent).map_err(|e| file_error("DXVK 대상 디렉터리 준비", e))?;
    if is_reparse(parent)? {
        return Err("DXVK 대상 디렉터리가 reparse point여서 교체하지 않았습니다".into());
    }
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| file_error("DXVK 대상 디렉터리 확인", error))?;
    let _parent_guard = lock_directory(parent, "DXVK 대상 디렉터리 고정")?;
    let parent_identity =
        file_identity(&_parent_guard, "DXVK 대상 디렉터리 identity 확인")?;
    if parent
        .canonicalize()
        .map_err(|error| file_error("DXVK 대상 디렉터리 재확인", error))?
        != canonical_parent
    {
        return Err("DXVK 대상 디렉터리가 검사 중 변경되었습니다".into());
    }
    if is_reparse(path)? {
        return Err("DXVK 대상 파일이 reparse point여서 교체하지 않았습니다".into());
    }
    let id = uuid::Uuid::new_v4();
    let temp = parent.join(format!(".dxvk-{id}.tmp"));
    let backup = parent.join(format!(".dxvk-backup-{id}.tmp"));
    let mut replaced = false;
    let mut backup_state = None;
    let mut target_guard = None;
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .access_mode(
                windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_READ
                    | windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_WRITE
                    | windows_sys::Win32::Storage::FileSystem::DELETE,
            )
            .share_mode(
                windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                    | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                    | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
            )
            .open(&temp)
            .map_err(|e| file_error(&format!("{action} 임시 파일 생성"), e))?;
        file.write_all(bytes)
            .map_err(|e| file_error(&format!("{action} 임시 파일 기록"), e))?;
        file.sync_all()
            .map_err(|e| file_error(&format!("{action} 임시 파일 동기화"), e))?;
        let staged_identity = file_identity(&file, &format!("{action} 임시 파일 identity 확인"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| file_error(&format!("{action} 임시 파일 재확인 준비"), e))?;
        let mut staged = Vec::with_capacity(bytes.len());
        file.read_to_end(&mut staged)
            .map_err(|e| file_error(&format!("{action} 임시 파일 재확인"), e))?;
        if http::sha256(&staged) != http::sha256(bytes) {
            return Err(format!("{action} 임시 파일 무결성 검증에 실패했습니다"));
        }
        if path.exists() {
            if is_reparse(path)? {
                return Err("DXVK 대상 파일이 교체 직전에 reparse point로 변경되었습니다".into());
            }
            let mut previous_file = fs::OpenOptions::new()
                .read(true)
                .share_mode(
                    windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                        | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE
                        | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_DELETE,
                )
                .open(path)
                .map_err(|e| file_error(&format!("{action} 기존 파일 열기"), e))?;
            let previous_identity =
                file_identity(&previous_file, &format!("{action} 기존 파일 identity 확인"))?;
            let mut previous = Vec::new();
            previous_file
                .read_to_end(&mut previous)
                .map_err(|e| file_error(&format!("{action} 기존 파일 읽기"), e))?;
            let mut backup_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&backup)
                .map_err(|e| file_error(&format!("{action} 기존 파일 백업 생성"), e))?;
            backup_file
                .write_all(&previous)
                .map_err(|e| file_error(&format!("{action} 기존 파일 백업 기록"), e))?;
            backup_file
                .sync_all()
                .map_err(|e| file_error(&format!("{action} 기존 파일 백업 동기화"), e))?;
            drop(backup_file);
            let saved = fs::read(&backup)
                .map_err(|e| file_error(&format!("{action} 기존 파일 백업 확인"), e))?;
            let hash = http::sha256(&previous);
            if http::sha256(&saved) != hash {
                return Err(format!("{action} 기존 파일 백업 무결성 검증에 실패했습니다"));
            }
            backup_state = Some((backup.clone(), hash));
            target_guard = Some((previous_file, previous_identity));
        }
        before_replace()?;
        if parent
            .canonicalize()
            .map_err(|error| file_error("DXVK 대상 디렉터리 최종 확인", error))?
            != canonical_parent
            || directory_path_identity(
                parent,
                "DXVK 대상 디렉터리 최종 identity 확인",
            )? != parent_identity
        {
            return Err("DXVK 대상 디렉터리가 교체 직전에 변경되었습니다".into());
        }
        if path_identity(&temp, &format!("{action} 임시 파일 최종 identity 확인"))?
            != staged_identity
        {
            return Err(format!("{action} 임시 파일이 검사 중 교체되었습니다"));
        }
        if let Some((_, expected_identity)) = &target_guard {
            if is_reparse(path)?
                || path_identity(path, &format!("{action} 기존 파일 최종 identity 확인"))?
                    != *expected_identity
            {
                return Err(format!("{action} 기존 파일이 검사 중 교체되었습니다"));
            }
        } else if path.exists() {
            return Err(format!("{action} 대상 파일이 검사 중 새로 생성되었습니다"));
        }
        drop(target_guard.take());
        handle_replace(&file, path, &format!("{action} 최종 파일 교체"))?;
        replaced = true;
        let written =
            fs::read(path).map_err(|e| file_error(&format!("{action} 최종 파일 확인"), e))?;
        if http::sha256(&written) != http::sha256(bytes) {
            return Err(format!("{action} 최종 파일 무결성 검증에 실패했습니다"));
        }
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    match result {
        Ok(()) => Ok(PendingReplacement {
            target: path.to_path_buf(),
            backup: backup_state,
            action: action.into(),
        }),
        Err(error) if replaced => Err(PendingReplacement {
            target: path.to_path_buf(),
            backup: backup_state,
            action: action.into(),
        }
        .rollback(error)),
        Err(error) => {
            let _ = fs::remove_file(&backup);
            Err(error)
        }
    }
}
fn atomic_bytes(path: &Path, bytes: &[u8], action: &str) -> Result<()> {
    atomic_bytes_guarded(path, bytes, action, || Ok(()))?.commit()
}
fn finish_apply(
    replacement: PendingReplacement,
    verify: impl FnOnce() -> Result<Value>,
) -> Result<Value> {
    let result = match verify() {
        Ok(result) => result,
        Err(error) => return Err(replacement.rollback(error)),
    };
    if result["matchesCurrent"] != true {
        return Err(replacement.rollback(
            "게임 폴더의 d3d9_dxvk.dll 적용 검증에 실패했습니다".into(),
        ));
    }
    replacement.commit()?;
    Ok(result)
}
pub fn apply(dir: &Path, target: &Path) -> Result<Value> {
    apply_guarded(dir, target, || Ok(()))
}
pub fn apply_guarded(
    dir: &Path,
    target: &Path,
    before_replace: impl FnOnce() -> Result<()>,
) -> Result<Value> {
    let current = installed(dir)?;
    if current["installed"] != true || current["integrity"] != true {
        return Err("게임에 적용할 검증된 DXVK 파일이 없습니다".into());
    }
    let bytes = fs::read(dir.join(current["current"]["fileName"].as_str().unwrap()))
        .map_err(|e| file_error("보관된 DXVK DLL 읽기", e))?;
    if http::sha256(&bytes) != current["current"]["sha256"] {
        return Err("게임 폴더 적용 전 DXVK 무결성 검증에 실패했습니다".into());
    }
    let replacement =
        atomic_bytes_guarded(target, &bytes, "게임 폴더 DXVK 적용", before_replace)?;
    finish_apply(replacement, || deployment(&current, target))
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
    atomic_bytes(&dir.join(&filename), &dll, "다운로드한 DXVK DLL 저장")?;
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
    use std::os::windows::fs::OpenOptionsExt;

    fn test_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nogirem-dxvk-test-{}", uuid::Uuid::new_v4()))
    }

    fn temporary_files(root: &Path) -> Vec<std::path::PathBuf> {
        fs::read_dir(root)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(".dxvk-"))
            })
            .collect()
    }

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
    #[test]
    fn atomic_bytes_creates_parent_and_replaces_existing_file() {
        let root = test_root();
        let target = root.join("nested/d3d9_dxvk.dll");
        assert!(!target.parent().unwrap().exists());
        atomic_bytes(&target, b"old", "DXVK 테스트 저장").unwrap();
        atomic_bytes(&target, b"new", "DXVK 테스트 저장").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_guard_preserves_existing_file_and_cleans_temporary_files() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, b"old").unwrap();
        let error = atomic_bytes_guarded(&target, b"new", "DXVK 테스트 저장", || {
            Err("교체 직전 검사 실패".into())
        })
        .unwrap_err();
        assert!(error.contains("교체 직전 검사 실패"));
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(temporary_files(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn locked_target_is_preserved_and_reports_in_use() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, b"old").unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
            .open(&target)
            .unwrap();
        let error = atomic_bytes(&target, b"new", "DXVK 테스트 저장").unwrap_err();
        assert!(error.contains("사용 여부"), "{error}");
        assert_eq!(fs::read(&target).unwrap(), b"old");
        drop(lock);
        assert!(temporary_files(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn windows_file_errors_have_actionable_categories() {
        assert!(
            file_error("DXVK 테스트", std::io::Error::from_raw_os_error(2))
                .contains("격리 여부")
        );
        assert!(
            file_error("DXVK 테스트", std::io::Error::from_raw_os_error(5))
                .contains("보안·백신")
        );
        assert!(
            file_error("DXVK 테스트", std::io::Error::from_raw_os_error(32))
                .contains("사용 중")
        );
    }
    #[test]
    fn post_replace_failure_restores_existing_file_and_cleans_backup() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, b"old").unwrap();
        let replacement =
            atomic_bytes_guarded(&target, b"new", "DXVK 테스트 저장", || Ok(())).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        let error = replacement.rollback("후속 검증 실패".into());
        assert_eq!(error, "후속 검증 실패");
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(temporary_files(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn restore_failure_preserves_verified_backup() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, b"old").unwrap();
        let replacement =
            atomic_bytes_guarded(&target, b"new", "DXVK 테스트 저장", || Ok(())).unwrap();
        let backup = replacement.backup.as_ref().unwrap().0.clone();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
            .open(&target)
            .unwrap();
        let error = replacement.rollback("후속 검증 실패".into());
        assert!(error.contains("수동 복구용"));
        assert_eq!(fs::read(&backup).unwrap(), b"old");
        drop(lock);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn post_replace_failure_without_original_removes_new_target() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        let replacement =
            atomic_bytes_guarded(&target, b"new", "DXVK 테스트 저장", || Ok(())).unwrap();
        let error = replacement.rollback("후속 검증 실패".into());
        assert_eq!(error, "후속 검증 실패");
        assert!(!target.exists());
        assert!(temporary_files(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn deployment_io_error_rolls_back_existing_target() {
        let root = test_root();
        let target = root.join("d3d9_dxvk.dll");
        fs::create_dir_all(&root).unwrap();
        fs::write(&target, b"old").unwrap();
        let replacement =
            atomic_bytes_guarded(&target, b"new", "DXVK 테스트 저장", || Ok(())).unwrap();
        let error = finish_apply(replacement, || Err("deployment I/O 오류".into())).unwrap_err();
        assert_eq!(error, "deployment I/O 오류");
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert!(temporary_files(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
