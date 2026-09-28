use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSet {
    pub group: u16,
    pub logical_processor_index: u8,
    pub core_index: u8,
    pub efficiency_class: u8,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub source: &'static str,
    pub all_mask: u64,
    pub game_mask: u64,
    pub background_mask: u64,
    pub alternate_game_mask: u64,
    pub alternate_background_mask: u64,
    pub last_performance_core_mask: u64,
    pub game_cpu_indexes: Vec<u8>,
    pub background_cpu_indexes: Vec<u8>,
    pub physical_core_count: Option<usize>,
    pub performance_core_count: Option<usize>,
    pub efficiency_core_count: Option<usize>,
    pub game_core_count: Option<usize>,
    pub default_game_core_count: Option<usize>,
    pub max_game_core_count: Option<usize>,
    pub hybrid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_error: Option<serde_json::Value>,
}
fn validate_count(n: usize) -> Result<()> {
    if !(4..=52).contains(&n) || n % 2 != 0 {
        Err(format!(
            "Unsupported logical CPU count: {n}; expected an even count from 4 to 52."
        ))
    } else {
        Ok(())
    }
}
pub fn default_game_core_count(performance: usize) -> Result<usize> {
    match performance {
        0 => Err("Invalid performance core count: 0".into()),
        1 => Ok(1),
        n => Ok((n - 1).min(4.max(n.div_ceil(2)))),
    }
}
pub fn mask(indexes: &[u8]) -> u64 {
    indexes.iter().fold(0, |m, i| m | (1u64 << i))
}
pub fn cpu_range(indexes: &[u8]) -> String {
    let Some(&first) = indexes.first() else {
        return "없음".into();
    };
    let mut ranges = Vec::new();
    let (mut start, mut end) = (first, first);
    let range = |s, e| {
        if s == e {
            format!("{s}")
        } else {
            format!("{s}-{e}")
        }
    };
    for &i in &indexes[1..] {
        if i == end + 1 {
            end = i;
        } else {
            ranges.push(range(start, end));
            start = i;
            end = i;
        }
    }
    ranges.push(range(start, end));
    ranges.join(",")
}
pub fn build(cpu_sets: &[CpuSet], count: usize, requested: Option<i64>) -> Result<Allocation> {
    validate_count(count)?;
    let mut processors: Vec<_> = cpu_sets.iter().filter(|p| p.group == 0).collect();
    processors.sort_by_key(|p| p.logical_processor_index);
    if processors.len() != count
        || processors
            .iter()
            .enumerate()
            .any(|(i, p)| p.logical_processor_index as usize != i)
    {
        return Err("CPU Set topology does not match the process affinity group.".into());
    }
    let mut cores: BTreeMap<u8, (u8, Vec<u8>)> = BTreeMap::new();
    for p in processors {
        let core = cores
            .entry(p.core_index)
            .or_insert((p.efficiency_class, Vec::new()));
        if core.0 != p.efficiency_class {
            return Err(format!(
                "Inconsistent efficiency class for physical core 0:{}.",
                p.core_index
            ));
        }
        core.1.push(p.logical_processor_index);
    }
    let mut cores: Vec<_> = cores.into_values().collect();
    cores.sort_by_key(|c| c.1[0]);
    let performance_class = cores.iter().map(|c| c.0).max().ok_or("No CPU cores")?;
    let performance: Vec<_> = cores.iter().filter(|c| c.0 == performance_class).collect();
    let efficiency: Vec<_> = cores.iter().filter(|c| c.0 < performance_class).collect();
    let default_count = default_game_core_count(performance.len())?;
    let maximum = performance.len().saturating_sub(1).max(1);
    let chosen = requested
        .map(|n| n.clamp(1, maximum as i64) as usize)
        .unwrap_or(default_count);
    let split = performance.len() - chosen;
    let hybrid = !efficiency.is_empty();
    let indexes = |list: Vec<&(u8, Vec<u8>)>| {
        let mut v: Vec<_> = list.iter().flat_map(|c| c.1.iter().copied()).collect();
        v.sort_unstable();
        v
    };
    let game = indexes(if hybrid {
        performance.clone()
    } else {
        performance[split..].to_vec()
    });
    let background = indexes(
        performance[..split]
            .iter()
            .chain(efficiency.iter())
            .copied()
            .collect(),
    );
    let alternate_game = indexes(performance[..split].to_vec());
    let alternate_background = indexes(
        performance[split..]
            .iter()
            .chain(efficiency.iter())
            .copied()
            .collect(),
    );
    Ok(Allocation {
        source: "windows-cpu-sets",
        all_mask: (1u64 << count) - 1,
        game_mask: mask(&game),
        background_mask: mask(&background),
        alternate_game_mask: mask(&alternate_game),
        alternate_background_mask: mask(&alternate_background),
        last_performance_core_mask: mask(&performance.last().unwrap().1),
        game_cpu_indexes: game,
        background_cpu_indexes: background,
        physical_core_count: Some(cores.len()),
        performance_core_count: Some(performance.len()),
        efficiency_core_count: Some(efficiency.len()),
        game_core_count: Some(chosen),
        default_game_core_count: Some(default_count),
        max_game_core_count: Some(maximum),
        hybrid: Some(hybrid),
        topology_error: None,
    })
}
pub fn fallback(count: usize, error: String) -> Result<Allocation> {
    validate_count(count)?;
    let half = count / 2;
    let all = (1u64 << count) - 1;
    let low = (1u64 << half) - 1;
    Ok(Allocation {
        source: "logical-half-fallback",
        all_mask: all,
        game_mask: all ^ low,
        background_mask: low,
        alternate_game_mask: low,
        alternate_background_mask: all ^ low,
        last_performance_core_mask: 3u64 << (count - 2),
        game_cpu_indexes: (half as u8..count as u8).collect(),
        background_cpu_indexes: (0..half as u8).collect(),
        physical_core_count: None,
        performance_core_count: None,
        efficiency_core_count: None,
        game_core_count: None,
        default_game_core_count: None,
        max_game_core_count: None,
        hybrid: None,
        topology_error: Some(serde_json::json!({"message":error,"logicalCpuCount":count})),
    })
}
pub fn query() -> Result<Vec<CpuSet>> {
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::{SystemInformation::GetSystemCpuSetInformation, Threading::GetCurrentProcess},
    };
    unsafe {
        let mut length = 0;
        let ok = GetSystemCpuSetInformation(
            std::ptr::null_mut(),
            0,
            &mut length,
            GetCurrentProcess(),
            0,
        );
        if ok == 0 && GetLastError() != 122 {
            return Err(format!(
                "GetSystemCpuSetInformation size query failed (Win32 {})",
                GetLastError()
            ));
        }
        if length == 0 {
            return Err("GetSystemCpuSetInformation returned no CPU Sets.".into());
        }
        // Use u64 storage to preserve the API structure alignment.
        let mut data = vec![0u64; (length as usize).div_ceil(8)];
        if GetSystemCpuSetInformation(
            data.as_mut_ptr().cast(),
            length,
            &mut length,
            GetCurrentProcess(),
            0,
        ) == 0
        {
            return Err(format!(
                "GetSystemCpuSetInformation failed (Win32 {})",
                GetLastError()
            ));
        }
        if length as usize > data.len() * 8 {
            return Err("CPU Set buffer grew unexpectedly".into());
        }
        let data = std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), length as usize);
        parse_cpu_sets(data)
    }
}
pub fn parse_cpu_sets(data: &[u8]) -> Result<Vec<CpuSet>> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
        if size < 8 || size > data.len() - offset {
            return Err("GetSystemCpuSetInformation returned malformed data.".into());
        }
        if kind == 0 && size >= 32 {
            result.push(CpuSet {
                group: u16::from_le_bytes(data[offset + 12..offset + 14].try_into().unwrap()),
                logical_processor_index: data[offset + 14],
                core_index: data[offset + 15],
                efficiency_class: data[offset + 18],
            });
        }
        offset += size;
    }
    Ok(result)
}
pub fn resolve(requested: Option<i64>) -> Result<Allocation> {
    use windows_sys::Win32::System::Threading::GetActiveProcessorCount;
    let count = unsafe { GetActiveProcessorCount(0xffff) } as usize;
    query()
        .and_then(|sets| build(&sets, count, requested))
        .or_else(|e| fallback(count, e))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_topology_is_rejected() {
        assert!(parse_cpu_sets(&[0; 8]).is_err());
        assert!(build(&[], 8, None).is_err());
        for count in [0, 2, 7, 54, 64] {
            assert!(fallback(count, "test".into()).is_err());
        }
    }
    #[test]
    fn hybrid_all_performance_cores_for_game() {
        let cpus: Vec<_> = (0..24)
            .map(|i| CpuSet {
                group: 0,
                logical_processor_index: i,
                core_index: if i < 16 { i / 2 } else { i - 8 },
                efficiency_class: if i < 16 { 1 } else { 0 },
            })
            .collect();
        let a = build(&cpus, 24, Some(4)).unwrap();
        assert_eq!(a.game_mask, 0xffff);
        assert_eq!(a.background_mask, 0xff00ff);
        assert_eq!(a.alternate_game_mask, 0xff);
        assert_eq!(a.alternate_background_mask, 0xffff00);
    }
}
