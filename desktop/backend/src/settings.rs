use serde_json::{Value, json};
pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+F10";
pub fn shortcut(value: &Value) -> String {
    if value.as_str().is_some_and(|s| s.trim().is_empty()) {
        return String::new();
    }
    let mut modifiers = Vec::new();
    let mut key = String::new();
    for token in value
        .as_str()
        .unwrap_or("")
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let lower = token.to_ascii_lowercase();
        let modifier = match lower.as_str() {
            "commandorcontrol" | "cmdorctrl" => Some("CommandOrControl"),
            "control" | "ctrl" => Some("Control"),
            "alt" => Some("Alt"),
            "shift" => Some("Shift"),
            "super" | "meta" => Some("Super"),
            _ => None,
        };
        if let Some(m) = modifier {
            if !modifiers.contains(&m) {
                modifiers.push(m);
            }
            continue;
        }
        if !key.is_empty() {
            return DEFAULT_SHORTCUT.into();
        }
        let upper = token.to_ascii_uppercase();
        if (upper.len() == 1 && upper.as_bytes()[0].is_ascii_alphanumeric())
            || upper.strip_prefix('F').is_some_and(|n| {
                n.parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)) && !n.starts_with('0')
            })
        {
            key = upper;
            continue;
        }
        key = match lower.as_str() {
            "space" => "Space",
            "up" => "Up",
            "down" => "Down",
            "left" => "Left",
            "right" => "Right",
            "insert" => "Insert",
            "delete" => "Delete",
            "home" => "Home",
            "end" => "End",
            "pageup" => "PageUp",
            "pagedown" => "PageDown",
            "pause" | "pausebreak" => "Pause",
            "printscreen" => "PrintScreen",
            "scrolllock" => "Scrolllock",
            "numlock" => "Numlock",
            "capslock" => "Capslock",
            "numadd" => "numadd",
            "numsub" => "numsub",
            "nummult" => "nummult",
            "numdiv" => "numdiv",
            "numdec" => "numdec",
            _ => return DEFAULT_SHORTCUT.into(),
        }
        .into();
    }
    if key == "Pause" {
        return key;
    }
    if key.is_empty()
        || (modifiers.is_empty()
            && !matches!(
                key.as_str(),
                "PrintScreen" | "Scrolllock" | "Numlock" | "Capslock"
            ))
    {
        return DEFAULT_SHORTCUT.into();
    }
    let order = ["CommandOrControl", "Control", "Alt", "Shift", "Super"];
    modifiers.sort_by_key(|m| order.iter().position(|o| o == m).unwrap());
    modifiers.push(&key);
    modifiers.join("+")
}
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if s.trim().is_empty() => Some(0.0),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}
fn choice(v: &Value, choices: &[i64], default: i64) -> i64 {
    let n = number(v).unwrap_or(f64::NAN);
    choices
        .iter()
        .find(|c| **c as f64 == n)
        .copied()
        .unwrap_or(default)
}
fn text_choice<'a>(v: &'a Value, choices: &[&str], default: &'a str) -> &'a str {
    v.as_str()
        .filter(|s| choices.contains(s))
        .unwrap_or(default)
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}
pub fn blackbox(v: &Value) -> Value {
    let drive = v["ringStorageDrive"]
        .as_str()
        .or_else(|| {
            let p = v["ringStoragePath"].as_str()?.trim();
            let b = p.as_bytes();
            (b.len() >= 3
                && b[0].is_ascii_alphabetic()
                && b[1] == b':'
                && matches!(b[2], b'/' | b'\\'))
            .then(|| &p[..2])
        })
        .unwrap_or("")
        .trim();
    let drive = if drive.len() == 2
        && drive.as_bytes()[0].is_ascii_alphabetic()
        && drive.as_bytes()[1] == b':'
    {
        drive.to_ascii_uppercase()
    } else {
        String::new()
    };
    let duration = v
        .get("maxDurationSeconds")
        .and_then(|v| if v.is_null() { Some(0.0) } else { number(v) })
        .filter(|n| n.is_finite())
        .map(|n| (n + 0.5).floor().clamp(60.0, 604800.0) as u64)
        .unwrap_or(3600);
    json!({"featureEnabled":truthy(&v["featureEnabled"])||truthy(&v["enabled"]),"enabled":truthy(&v["enabled"]),
        "codec":text_choice(&v["codec"],&["h264","hevc"],"h264"),"quality":text_choice(&v["quality"],&["auto","1080p","1440p","original"],"auto"),
        "capacityGb":choice(&v["capacityGb"],&[20,50,100,200],50),"maxDurationSeconds":duration,"ringStorageDrive":drive,
        "clipStoragePath":v["clipStoragePath"].as_str().unwrap_or("").trim(),"clipSeconds":choice(&v["clipSeconds"],&[30,60,120],30),
        "fps":choice(&v["fps"],&[30,60],60),"chunkSeconds":10,"shortcut":shortcut(&v["shortcut"])})
}
pub fn quality(v: &Value, logical_cpus: u64, total_memory: u64) -> String {
    let s = blackbox(v);
    let q = s["quality"].as_str().unwrap();
    if q == "auto" {
        if logical_cpus >= 12 && total_memory >= 16 * 1024 * 1024 * 1024 {
            "1440p"
        } else {
            "1080p"
        }
    } else {
        q
    }
    .into()
}
pub fn bitrate(v: &Value) -> u32 {
    let s = blackbox(v);
    let q = s["quality"].as_str().unwrap();
    let hevc = s["codec"] == "hevc";
    let sixty = s["fps"] == 60;
    match (q, hevc, sixty) {
        ("1080p", false, true) => 12,
        ("1080p", false, false) => 7,
        ("1080p", true, true) => 8,
        ("1080p", true, false) => 5,
        ("original", false, true) => 32,
        ("original", false, false) => 20,
        ("original", true, true) => 22,
        ("original", true, false) => 14,
        (_, false, true) => 24,
        (_, false, false) => 14,
        (_, true, true) => 16,
        (_, true, false) => 10,
    }
}
pub fn turbo_keys(v: &Value) -> Vec<u64> {
    let mut keys:Vec<_>=v.as_array().into_iter().flatten().filter_map(Value::as_u64).filter(|n|matches!(n,0x08|0x09|0x0d|0x20|0x21..=0x28|0x2d|0x2e|0x30..=0x39|0x41..=0x5a|0x60..=0x7b|0xba..=0xc0|0xdb..=0xde)).collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}
pub fn turbo(v: &Value) -> Value {
    json!({"enabled":v["enabled"]==true,"keys":turbo_keys(&v["keys"]),"intervalMs":choice(&v["intervalMs"],&[1,3,5,10,20,30],1),"ignoreInitialDelay":v["ignoreInitialDelay"]==true})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcut_edge_cases() {
        for (input, expected) in [
            ("", ""),
            ("shift+ctrl+f10", "Control+Shift+F10"),
            ("ctrl+PauseBreak", "Pause"),
            ("A", DEFAULT_SHORTCUT),
            ("ctrl+A+B", DEFAULT_SHORTCUT),
            ("Meta+Meta+F24", "Super+F24"),
        ] {
            assert_eq!(shortcut(&json!(input)), expected);
        }
    }
    #[test]
    fn migration_preserves_legacy_drive_and_limits() {
        let s = blackbox(
            &json!({"ringStoragePath":"d:\\old\\ring","enabled":true,"maxDurationSeconds":999999999,"capacityGb":40}),
        );
        assert_eq!(s["ringStorageDrive"], "D:");
        assert_eq!(s["featureEnabled"], true);
        assert_eq!(s["maxDurationSeconds"], 604800);
        assert_eq!(s["capacityGb"], 50);
    }
}
