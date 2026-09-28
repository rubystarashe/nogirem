use crate::Result;
use base64::Engine;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};
fn utf16(bytes: &[u8]) -> Result<String> {
    if bytes.len() % 2 != 0 {
        return Err("UTF-16LE 길이 오류".into());
    }
    String::from_utf16(
        &bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| e.to_string())
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffffffffu32;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    c ^ 0xffffffff
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn hex(s: &str) -> Result<Vec<u8>> {
    if s.is_empty()
        || s.len() % 2 != 0
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("압축 데이터가 유효한 소문자 16진수가 아닙니다.".into());
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).map_err(|e| e.to_string()))
        .collect()
}
pub fn decode(bytes: &[u8]) -> Result<Value> {
    let (format, inner, stored_crc) = if bytes.starts_with(b"MUO2") {
        if bytes.len() < 0x24 {
            return Err("MUO2 파일이 너무 짧습니다.".into());
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != 1 {
            return Err(format!("지원하지 않는 MUO2 버전: {version}"));
        }
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        let count = u32_at(bytes, 8) as usize;
        if count != u32_at(bytes, 32) as usize || bytes.len() != 0x24 + count * 2 {
            return Err("MUO2 문자열 길이와 파일 크기가 일치하지 않습니다.".into());
        }
        let text = utf16(&bytes[0x24..])?;
        let inner = if flags & 2 != 0 {
            use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
            use sha2::{Digest, Sha256};
            let mut key = Sha256::new();
            key.update(hex("7a145a9f319947b9faaf2488b8bbbe63")?);
            key.update(b"MUO_FILE_KEY_V1");
            let key = key.finalize();
            let mut cipher = base64::engine::general_purpose::STANDARD
                .decode(text)
                .map_err(|e| e.to_string())?;
            let plain = cbc::Decryptor::<aes::Aes128>::new_from_slices(&key[..16], &bytes[16..32])
                .map_err(|e| e.to_string())?
                .decrypt_padded_mut::<Pkcs7>(&mut cipher)
                .map_err(|e| e.to_string())?;
            String::from_utf8(plain.to_vec()).map_err(|e| e.to_string())?
        } else {
            text
        };
        ("muo2", inner, u32_at(bytes, 12))
    } else {
        if bytes.len() < 8 {
            return Err("MUO 파일이 너무 짧습니다.".into());
        }
        let count = u32_at(bytes, 4) as usize;
        if bytes.len() != 8 + count * 2 {
            return Err(format!(
                "MUO 길이 불일치: 헤더 문자 수={count}, 파일 크기={}",
                bytes.len()
            ));
        }
        ("legacyMuo", utf16(&bytes[8..])?, u32_at(bytes, 0))
    };
    let crc_bytes: Vec<u8> = inner
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let crc = crc32(&crc_bytes);
    if crc != stored_crc {
        return Err(format!(
            "CRC32 불일치: 저장=0x{stored_crc:x}, 계산=0x{crc:x}"
        ));
    }
    let (size, compressed) = inner
        .split_once(';')
        .ok_or("MUO 내부 헤더가 올바르지 않습니다.")?;
    if size.is_empty() || !size.bytes().all(|b| b.is_ascii_digit()) {
        return Err("XML 크기 필드를 읽을 수 없습니다.".into());
    }
    let size: usize = size.parse().map_err(|_| "XML 크기가 올바르지 않습니다")?;
    if size > 32 * 1024 * 1024 {
        return Err("MUO XML 허용 크기 초과".into());
    }
    let compressed = hex(compressed)?;
    let mut xml = Vec::new();
    flate2::read::ZlibDecoder::new(compressed.as_slice())
        .take(size as u64 + 1)
        .read_to_end(&mut xml)
        .map_err(|e| e.to_string())?;
    if xml.len() != size {
        return Err(format!("XML 크기 불일치: 예상={size}, 실제={}", xml.len()));
    }
    if !xml.ends_with(&[0, 0]) {
        return Err("UTF-16LE 종료 문자가 없습니다.".into());
    }
    let text = utf16(&xml[..xml.len() - 2])?;
    Ok(json!({"format":format,"xmlText":text.trim_start_matches('\u{feff}')}))
}
pub fn latest(dir: &Path) -> Result<Value> {
    let mut files = vec![];
    for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_file()
            && e.path()
                .extension()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case("muo"))
        {
            let time = e
                .metadata()
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?;
            files.push((time, e.file_name(), e.path()));
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    let (time, name, path) = files
        .first()
        .ok_or_else(|| format!(".muo 파일이 없습니다: {}", dir.display()))?;
    let decoded = decode(&fs::read(path).map_err(|e| e.to_string())?)?;
    let text = decoded["xmlText"].as_str().unwrap();
    let pattern =
        regex::Regex::new(r#"(?i)\bDummyCharRenderModeFPS\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap();
    let matches = pattern
        .captures(text)
        .ok_or("DummyCharRenderModeFPS 속성을 찾을 수 없습니다.")?;
    let value = matches.get(1).or(matches.get(2)).unwrap().as_str();
    let modified: chrono::DateTime<chrono::Utc> = (*time).into();
    Ok(
        json!({"applied":value=="-1","attribute":"DummyCharRenderModeFPS","value":value,"format":decoded["format"],"fileName":name.to_string_lossy(),"filePath":path.to_string_lossy(),"modifiedAt":modified.to_rfc3339_opts(chrono::SecondsFormat::Millis,true)}),
    )
}
pub fn install(root: &Path, documents: &Path, name: &str) -> Result<Value> {
    if Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name)
        || name.contains(['/', '\\', ':'])
    {
        return Err("잘못된 간소화 파일명".into());
    }
    let source = fs::read(root.join("assets").join(name)).map_err(|e| e.to_string())?;
    let directory = documents.join("마비노기/설정");
    let target = directory.join("목록").join(name);
    fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&target, &source).map_err(|e| e.to_string())?;
    if fs::read(target).map_err(|e| e.to_string())? != source {
        return Err("주변 캐릭터 간소화 파일 복사 검증에 실패했습니다".into());
    }
    match fs::remove_file(directory.join(name)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    Ok(json!(true))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_files_rejected() {
        assert!(decode(b"MUO2").is_err());
        assert!(decode(&[0; 8]).is_err());
        assert!(decode(&[0; 10]).is_err());
    }
    #[test]
    fn installs_exact_bytes_and_removes_legacy_copy() {
        let root = std::env::temp_dir().join(format!("nogirem-muo-test-{}", uuid::Uuid::new_v4()));
        let docs = root.join("documents");
        let settings = docs.join("마비노기/설정");
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::create_dir_all(&settings).unwrap();
        let name = "test.muo";
        let source = include_bytes!("../../../assets/주변캐릭터간소화프레임제한해제.muo");
        fs::write(root.join("assets").join(name), source).unwrap();
        fs::write(settings.join(name), b"old").unwrap();
        install(&root, &docs, name).unwrap();
        assert_eq!(fs::read(settings.join("목록").join(name)).unwrap(), source);
        assert!(!settings.join(name).exists());
        assert!(install(&root, &docs, "../test.muo").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
