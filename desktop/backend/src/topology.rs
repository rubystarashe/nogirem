use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use windows_sys::Win32::System::SystemInformation::{
    CACHE_RELATIONSHIP, GROUP_AFFINITY, GROUP_RELATIONSHIP, PROCESSOR_GROUP_INFO,
    PROCESSOR_RELATIONSHIP, RelationCache, RelationGroup, RelationProcessorCore,
    SYSTEM_CPU_SET_INFORMATION,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSet {
    pub group: u16,
    pub logical_processor_index: u8,
    pub core_index: u8,
    #[serde(default)]
    pub last_level_cache_index: u8,
    pub efficiency_class: u8,
    #[serde(default)]
    pub scheduling_class: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroupMask {
    group: u16,
    mask: u64,
}

#[derive(Clone, Debug, Default)]
struct LogicalTopology {
    cores: Vec<Vec<GroupMask>>,
    l3_caches: Vec<Vec<GroupMask>>,
    groups: Vec<GroupMask>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub source: &'static str,
    pub supported: bool,
    pub processor_group_count: usize,
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
    pub balanced_game_core_count: Option<usize>,
    pub focus_game_core_count: Option<usize>,
    pub max_game_core_count: Option<usize>,
    pub hybrid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_error: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
struct Core {
    mask: u64,
    first: u8,
    efficiency_class: u8,
    scheduling_class: u8,
    l3_domain: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuMode {
    Focus,
    Balanced,
    Safe,
}

impl Default for CpuMode {
    fn default() -> Self {
        Self::Balanced
    }
}

impl CpuMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "focus" => Self::Focus,
            "safe" => Self::Safe,
            _ => Self::Balanced,
        }
    }
}

pub fn default_game_core_count(performance: usize) -> Result<usize> {
    match performance {
        0 => Err("Invalid performance core count: 0".into()),
        1 => Ok(1),
        n => Ok((n - 1).min(4.max(n.div_ceil(2)))),
    }
}

pub fn focus_game_core_count(performance: usize) -> Result<usize> {
    let balanced = default_game_core_count(performance)?;
    Ok(performance.min(balanced.saturating_add(2)))
}

pub fn mask(indexes: &[u8]) -> u64 {
    indexes
        .iter()
        .filter(|index| **index < 64)
        .fold(0, |value, index| value | (1u64 << index))
}

pub fn cpu_range(indexes: &[u8]) -> String {
    let Some(&first) = indexes.first() else {
        return "없음".into();
    };
    let mut ranges = Vec::new();
    let (mut start, mut end) = (first, first);
    let range = |start, end| {
        if start == end {
            format!("{start}")
        } else {
            format!("{start}-{end}")
        }
    };
    for &index in &indexes[1..] {
        if index == end + 1 {
            end = index;
        } else {
            ranges.push(range(start, end));
            start = index;
            end = index;
        }
    }
    ranges.push(range(start, end));
    ranges.join(",")
}

fn mask_indexes(value: u64) -> Vec<u8> {
    (0..64)
        .filter(|index| value & (1u64 << index) != 0)
        .collect()
}

fn unsupported(
    logical_count: usize,
    group_count: usize,
    counts: Option<(usize, usize, usize, bool)>,
    error: String,
) -> Allocation {
    let all_mask = match logical_count {
        0 => 0,
        64.. => u64::MAX,
        count => (1u64 << count) - 1,
    };
    let (physical, performance, efficiency, hybrid) = counts
        .map(|(physical, performance, efficiency, hybrid)| {
            (
                Some(physical),
                Some(performance),
                Some(efficiency),
                Some(hybrid),
            )
        })
        .unwrap_or((None, None, None, None));
    Allocation {
        source: "windows-cpu-sets+glpiex",
        supported: false,
        processor_group_count: group_count,
        all_mask,
        game_mask: all_mask,
        background_mask: all_mask,
        alternate_game_mask: 0,
        alternate_background_mask: all_mask,
        last_performance_core_mask: 0,
        game_cpu_indexes: Vec::new(),
        background_cpu_indexes: mask_indexes(all_mask),
        physical_core_count: physical,
        performance_core_count: performance,
        efficiency_core_count: efficiency,
        game_core_count: None,
        default_game_core_count: None,
        balanced_game_core_count: None,
        focus_game_core_count: None,
        max_game_core_count: None,
        hybrid,
        topology_error: Some(serde_json::json!({
            "message": error,
            "logicalCpuCount": logical_count,
            "processorGroupCount": group_count
        })),
    }
}

fn selected_by_l3(cores: &[Core], count: usize) -> Vec<Core> {
    let mut domains: BTreeMap<usize, Vec<Core>> = BTreeMap::new();
    for core in cores {
        domains
            .entry(core.l3_domain)
            .or_default()
            .push(core.clone());
    }
    let mut domains: Vec<_> = domains.into_values().collect();
    for domain in &mut domains {
        domain.sort_by_key(|core| (std::cmp::Reverse(core.scheduling_class), core.first));
    }
    domains.sort_by_key(|domain| {
        (
            std::cmp::Reverse(domain.len()),
            domain.iter().map(|core| core.first).min().unwrap_or(u8::MAX),
        )
    });
    domains
        .into_iter()
        .flatten()
        .take(count)
        .collect()
}

fn build(
    cpu_sets: &[CpuSet],
    topology: &LogicalTopology,
    requested: Option<i64>,
    mode: CpuMode,
) -> Result<Allocation> {
    let group_count = topology.groups.len();
    if group_count != 1 || topology.groups[0].group != 0 {
        let logical_count = topology
            .groups
            .iter()
            .map(|group| group.mask.count_ones() as usize)
            .sum();
        return Ok(unsupported(
            logical_count,
            group_count,
            None,
            format!(
                "복수 processor group({group_count}개)은 단일 affinity mask로 안전하게 표현할 수 없습니다"
            ),
        ));
    }
    let active_mask = topology.groups[0].mask;
    let logical_count = active_mask.count_ones() as usize;
    if logical_count == 0 || logical_count > 64 {
        return Err("활성 logical processor 수가 올바르지 않습니다".into());
    }
    let sets: BTreeMap<_, _> = cpu_sets
        .iter()
        .map(|set| ((set.group, set.logical_processor_index), set))
        .collect();
    if sets.len() != logical_count
        || sets
            .keys()
            .any(|(group, logical)| *group != 0 || active_mask & (1u64 << logical) == 0)
    {
        return Err("CPU Set topology does not match RelationGroup.".into());
    }
    let mut covered = 0u64;
    let mut cores = Vec::new();
    for masks in &topology.cores {
        if masks.len() != 1 || masks[0].group != 0 || masks[0].mask == 0 {
            return Err("RelationProcessorCore contains an unsupported group mask.".into());
        }
        let core_mask = masks[0].mask;
        if core_mask & !active_mask != 0 || covered & core_mask != 0 {
            return Err("RelationProcessorCore masks overlap or exceed RelationGroup.".into());
        }
        let logicals = mask_indexes(core_mask);
        let members: Vec<_> = logicals
            .iter()
            .map(|logical| {
                sets.get(&(0, *logical))
                    .copied()
                    .ok_or_else(|| "RelationProcessorCore is missing matching CPU Set.".to_string())
            })
            .collect::<std::result::Result<_, _>>()?;
        let efficiency = members[0].efficiency_class;
        let scheduling = members[0].scheduling_class;
        if members
            .iter()
            .any(|set| {
                set.efficiency_class != efficiency || set.scheduling_class != scheduling
            })
        {
            return Err("Inconsistent efficiency or scheduling class for physical core.".into());
        }
        let domains: Vec<_> = topology
            .l3_caches
            .iter()
            .enumerate()
            .filter(|(_, masks)| {
                masks
                    .iter()
                    .any(|mask| mask.group == 0 && mask.mask & core_mask == core_mask)
            })
            .map(|(index, _)| index)
            .collect();
        let l3_domain = match domains.as_slice() {
            [domain] => *domain,
            [] => {
                let indexes: BTreeSet<_> = members
                    .iter()
                    .map(|set| set.last_level_cache_index)
                    .collect();
                if indexes.len() != 1 {
                    return Err("CPU Set L3 indexes are inconsistent within a physical core.".into());
                }
                topology.l3_caches.len() + usize::from(*indexes.iter().next().unwrap())
            }
            _ => return Err("A physical core belongs to multiple L3 cache domains.".into()),
        };
        cores.push(Core {
            mask: core_mask,
            first: logicals[0],
            efficiency_class: efficiency,
            scheduling_class: scheduling,
            l3_domain,
        });
        covered |= core_mask;
    }
    if covered != active_mask {
        return Err("RelationProcessorCore does not cover every active logical processor.".into());
    }
    cores.sort_by_key(|core| core.first);
    let performance_class = cores
        .iter()
        .map(|core| core.efficiency_class)
        .max()
        .ok_or("No physical CPU cores")?;
    let performance: Vec<_> = cores
        .iter()
        .filter(|core| core.efficiency_class == performance_class)
        .cloned()
        .collect();
    let efficiency_count = cores.len() - performance.len();
    let hybrid = efficiency_count > 0;
    let counts = Some((cores.len(), performance.len(), efficiency_count, hybrid));
    if performance.len() == 1 && !hybrid {
        return Ok(unsupported(
            logical_count,
            group_count,
            counts,
            "단일 물리 코어 시스템에서는 CPU 분할을 사용하지 않습니다".into(),
        ));
    }
    let balanced_count = default_game_core_count(performance.len())?;
    let focus_count = focus_game_core_count(performance.len())?;
    let default_count = if mode == CpuMode::Focus {
        focus_count
    } else {
        balanced_count
    };
    let maximum = if hybrid {
        performance.len()
    } else {
        performance.len() - 1
    };
    let chosen = requested
        .map(|value| value.clamp(1, maximum as i64) as usize)
        .unwrap_or(default_count.min(maximum));
    let game_cores = selected_by_l3(&performance, chosen);
    let game_mask = game_cores.iter().fold(0, |mask, core| mask | core.mask);
    let mut background_mask = active_mask & !game_mask;
    if hybrid && performance.iter().all(|core| core.mask & background_mask == 0) {
        background_mask |= game_cores
            .last()
            .ok_or("No selected performance core")?
            .mask;
    }
    if game_mask == 0 || background_mask == 0 {
        return Err("CPU allocation did not produce usable game and background masks.".into());
    }
    let remaining_performance: Vec<_> = performance
        .iter()
        .filter(|core| core.mask & game_mask == 0)
        .cloned()
        .collect();
    let alternate = if remaining_performance.len() >= chosen {
        selected_by_l3(&remaining_performance, chosen)
    } else {
        Vec::new()
    };
    let alternate_game_mask = alternate.iter().fold(0, |mask, core| mask | core.mask);
    let alternate_background_mask = if alternate_game_mask == 0 {
        active_mask
    } else {
        active_mask & !alternate_game_mask
    };
    Ok(Allocation {
        source: "windows-cpu-sets+glpiex",
        supported: true,
        processor_group_count: group_count,
        all_mask: active_mask,
        game_mask,
        background_mask,
        alternate_game_mask,
        alternate_background_mask,
        last_performance_core_mask: performance.last().map_or(0, |core| core.mask),
        game_cpu_indexes: mask_indexes(game_mask),
        background_cpu_indexes: mask_indexes(background_mask),
        physical_core_count: Some(cores.len()),
        performance_core_count: Some(performance.len()),
        efficiency_core_count: Some(efficiency_count),
        game_core_count: Some(chosen),
        default_game_core_count: Some(default_count),
        balanced_game_core_count: Some(balanced_count),
        focus_game_core_count: Some(focus_count.min(maximum)),
        max_game_core_count: Some(maximum),
        hybrid: Some(hybrid),
        topology_error: None,
    })
}

#[doc(hidden)]
pub fn build_fixture(
    cpu_sets: &[CpuSet],
    logical_count: usize,
    requested: Option<i64>,
) -> Result<Allocation> {
    if cpu_sets.len() != logical_count {
        return Err("CPU Set fixture count does not match its logical processors.".into());
    }
    let mut cores: BTreeMap<(u16, u8), u64> = BTreeMap::new();
    let mut caches: BTreeMap<(u16, u8), u64> = BTreeMap::new();
    let mut groups: BTreeMap<u16, u64> = BTreeMap::new();
    for set in cpu_sets {
        if set.logical_processor_index >= 64 {
            return Err("CPU Set fixture logical processor is out of range.".into());
        }
        let bit = 1u64 << set.logical_processor_index;
        *cores.entry((set.group, set.core_index)).or_default() |= bit;
        *caches
            .entry((set.group, set.last_level_cache_index))
            .or_default() |= bit;
        *groups.entry(set.group).or_default() |= bit;
    }
    build(
        cpu_sets,
        &LogicalTopology {
            cores: cores
                .into_iter()
                .map(|((group, _), mask)| vec![GroupMask { group, mask }])
                .collect(),
            l3_caches: caches
                .into_iter()
                .map(|((group, _), mask)| vec![GroupMask { group, mask }])
                .collect(),
            groups: groups
                .into_iter()
                .map(|(group, mask)| GroupMask { group, mask })
                .collect(),
        },
        requested,
        CpuMode::Balanced,
    )
}

fn checked_record(data: &[u8], offset: usize) -> Result<(i32, usize)> {
    if data.len().saturating_sub(offset) < 8 {
        return Err("GetLogicalProcessorInformationEx returned truncated data.".into());
    }
    let relationship = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    let size = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
    if size < 8 || size > data.len() - offset {
        return Err("GetLogicalProcessorInformationEx returned malformed data.".into());
    }
    Ok((relationship, size))
}

fn read_group_masks(
    data: &[u8],
    start: usize,
    count: usize,
    end: usize,
) -> Result<Vec<GroupMask>> {
    let size = std::mem::size_of::<GROUP_AFFINITY>();
    if count == 0 || start.saturating_add(count.saturating_mul(size)) > end {
        return Err("GetLogicalProcessorInformationEx group mask array is malformed.".into());
    }
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let raw = unsafe {
            std::ptr::read_unaligned(
                data.as_ptr()
                    .add(start + index * size)
                    .cast::<GROUP_AFFINITY>(),
            )
        };
        result.push(GroupMask {
            group: raw.Group,
            mask: raw.Mask as u64,
        });
    }
    Ok(result)
}

fn parse_relationships(data: &[u8], expected: i32) -> Result<Vec<Vec<GroupMask>>> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (relationship, size) = checked_record(data, offset)?;
        if relationship != expected {
            return Err("GetLogicalProcessorInformationEx returned an unexpected relationship.".into());
        }
        let end = offset + size;
        let (count, masks_start) = if expected == RelationProcessorCore {
            let base = offset + 8;
            let header = std::mem::size_of::<PROCESSOR_RELATIONSHIP>()
                - std::mem::size_of::<GROUP_AFFINITY>();
            if base + header > end {
                return Err("RelationProcessorCore record is truncated.".into());
            }
            let relation = unsafe {
                std::ptr::read_unaligned(data.as_ptr().add(base).cast::<PROCESSOR_RELATIONSHIP>())
            };
            (
                usize::from(relation.GroupCount),
                base + std::mem::offset_of!(PROCESSOR_RELATIONSHIP, GroupMask),
            )
        } else if expected == RelationCache {
            let base = offset + 8;
            let header = std::mem::size_of::<CACHE_RELATIONSHIP>()
                - std::mem::size_of::<GROUP_AFFINITY>();
            if base + header > end {
                return Err("RelationCache record is truncated.".into());
            }
            let relation = unsafe {
                std::ptr::read_unaligned(data.as_ptr().add(base).cast::<CACHE_RELATIONSHIP>())
            };
            if relation.Level != 3 {
                offset = end;
                continue;
            }
            (
                usize::from(relation.GroupCount),
                base + std::mem::offset_of!(CACHE_RELATIONSHIP, Anonymous),
            )
        } else {
            return Err("Unsupported logical processor relationship parser.".into());
        };
        result.push(read_group_masks(data, masks_start, count, end)?);
        offset = end;
    }
    if result.is_empty() && expected == RelationProcessorCore {
        return Err("GetLogicalProcessorInformationEx returned no physical cores.".into());
    }
    Ok(result)
}

fn parse_groups(data: &[u8]) -> Result<Vec<GroupMask>> {
    let (relationship, size) = checked_record(data, 0)?;
    if relationship != RelationGroup || size != data.len() {
        return Err("RelationGroup record is missing or malformed.".into());
    }
    let base = 8;
    let header =
        std::mem::size_of::<GROUP_RELATIONSHIP>() - std::mem::size_of::<PROCESSOR_GROUP_INFO>();
    if base + header > size {
        return Err("RelationGroup record is truncated.".into());
    }
    let relation = unsafe {
        std::ptr::read_unaligned(data.as_ptr().add(base).cast::<GROUP_RELATIONSHIP>())
    };
    let count = usize::from(relation.ActiveGroupCount);
    let start = base + std::mem::offset_of!(GROUP_RELATIONSHIP, GroupInfo);
    let info_size = std::mem::size_of::<PROCESSOR_GROUP_INFO>();
    if count == 0 || start + count * info_size > size {
        return Err("RelationGroup processor array is malformed.".into());
    }
    let mut groups = Vec::with_capacity(count);
    for index in 0..count {
        let info = unsafe {
            std::ptr::read_unaligned(
                data.as_ptr()
                    .add(start + index * info_size)
                    .cast::<PROCESSOR_GROUP_INFO>(),
            )
        };
        if info.ActiveProcessorCount as u32 != info.ActiveProcessorMask.count_ones() {
            return Err("RelationGroup active processor count does not match its mask.".into());
        }
        groups.push(GroupMask {
            group: index as u16,
            mask: info.ActiveProcessorMask as u64,
        });
    }
    Ok(groups)
}

pub fn parse_cpu_sets(data: &[u8]) -> Result<Vec<CpuSet>> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        if data.len() - offset < 8 {
            return Err("GetSystemCpuSetInformation returned truncated data.".into());
        }
        let size = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = i32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
        if size < 8 || size > data.len() - offset {
            return Err("GetSystemCpuSetInformation returned malformed data.".into());
        }
        if kind == 0 {
            if size < std::mem::size_of::<SYSTEM_CPU_SET_INFORMATION>() {
                return Err("GetSystemCpuSetInformation CPU Set record is truncated.".into());
            }
            let raw = unsafe {
                std::ptr::read_unaligned(
                    data.as_ptr()
                        .add(offset)
                        .cast::<SYSTEM_CPU_SET_INFORMATION>(),
                )
            };
            let set = unsafe { raw.Anonymous.CpuSet };
            result.push(CpuSet {
                group: set.Group,
                logical_processor_index: set.LogicalProcessorIndex,
                core_index: set.CoreIndex,
                last_level_cache_index: set.LastLevelCacheIndex,
                efficiency_class: set.EfficiencyClass,
                scheduling_class: unsafe { set.Anonymous2.SchedulingClass },
            });
        }
        offset += size;
    }
    if result.is_empty() {
        return Err("GetSystemCpuSetInformation returned no CPU Sets.".into());
    }
    Ok(result)
}

fn query_cpu_sets() -> Result<Vec<CpuSet>> {
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::{SystemInformation::GetSystemCpuSetInformation, Threading::GetCurrentProcess},
    };
    unsafe {
        let mut length = 0;
        if GetSystemCpuSetInformation(
            std::ptr::null_mut(),
            0,
            &mut length,
            GetCurrentProcess(),
            0,
        ) != 0
            || GetLastError() != 122
        {
            return Err(format!(
                "GetSystemCpuSetInformation size query failed (Win32 {})",
                GetLastError()
            ));
        }
        if length == 0 {
            return Err("GetSystemCpuSetInformation returned no CPU Sets.".into());
        }
        let mut storage = vec![0u64; (length as usize).div_ceil(8)];
        if GetSystemCpuSetInformation(
            storage.as_mut_ptr().cast(),
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
        let bytes =
            std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), length as usize);
        parse_cpu_sets(bytes)
    }
}

fn query_relationship(relationship: i32) -> Result<Vec<u8>> {
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::SystemInformation::GetLogicalProcessorInformationEx,
    };
    unsafe {
        let mut length = 0;
        if GetLogicalProcessorInformationEx(relationship, std::ptr::null_mut(), &mut length) != 0
            || GetLastError() != 122
        {
            return Err(format!(
                "GetLogicalProcessorInformationEx size query failed (Win32 {})",
                GetLastError()
            ));
        }
        if length == 0 {
            return Err("GetLogicalProcessorInformationEx returned no data.".into());
        }
        let mut storage = vec![0u64; (length as usize).div_ceil(8)];
        if GetLogicalProcessorInformationEx(
            relationship,
            storage.as_mut_ptr().cast(),
            &mut length,
        ) == 0
        {
            return Err(format!(
                "GetLogicalProcessorInformationEx failed (Win32 {})",
                GetLastError()
            ));
        }
        Ok(std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), length as usize).to_vec())
    }
}

fn query_topology() -> Result<LogicalTopology> {
    Ok(LogicalTopology {
        cores: parse_relationships(
            &query_relationship(RelationProcessorCore)?,
            RelationProcessorCore,
        )?,
        l3_caches: parse_relationships(&query_relationship(RelationCache)?, RelationCache)?,
        groups: parse_groups(&query_relationship(RelationGroup)?)?,
    })
}

pub fn resolve_mode(requested: Option<i64>, mode: CpuMode) -> Result<Allocation> {
    use windows_sys::Win32::System::Threading::{
        GetActiveProcessorCount, GetActiveProcessorGroupCount,
    };
    let group_count = unsafe { GetActiveProcessorGroupCount() } as usize;
    let logical_count = unsafe { GetActiveProcessorCount(0xffff) } as usize;
    match query_cpu_sets().and_then(|sets| {
        let topology = query_topology()?;
        build(&sets, &topology, requested, mode)
    }) {
        Ok(allocation) => Ok(allocation),
        Err(error) => Ok(unsupported(logical_count, group_count, None, error)),
    }
}

pub fn resolve(requested: Option<i64>) -> Result<Allocation> {
    resolve_mode(requested, CpuMode::Balanced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_mode_default는_균형이다() {
        assert_eq!(CpuMode::default(), CpuMode::Balanced)
    }

    fn cpu_sets(core_masks: &[u64], performance_cores: usize, l3_split: usize) -> Vec<CpuSet> {
        core_masks
            .iter()
            .enumerate()
            .flat_map(|(core, core_mask)| {
                mask_indexes(*core_mask).into_iter().map(move |logical| CpuSet {
                    group: 0,
                    logical_processor_index: logical,
                    core_index: core as u8,
                    last_level_cache_index: usize::from(core >= l3_split) as u8,
                    efficiency_class: u8::from(core < performance_cores),
                    scheduling_class: u8::from(core < performance_cores),
                })
            })
            .collect()
    }

    fn topology(core_masks: &[u64], l3_masks: &[u64]) -> LogicalTopology {
        LogicalTopology {
            cores: core_masks
                .iter()
                .map(|mask| vec![GroupMask { group: 0, mask: *mask }])
                .collect(),
            l3_caches: l3_masks
                .iter()
                .map(|mask| vec![GroupMask { group: 0, mask: *mask }])
                .collect(),
            groups: vec![GroupMask {
                group: 0,
                mask: core_masks.iter().fold(0, |all, mask| all | mask),
            }],
        }
    }

    #[test]
    fn hybrid_uses_only_requested_p_cores_and_keeps_masks_disjoint() {
        let core_masks: Vec<_> = (0..8)
            .map(|core| 3u64 << (core * 2))
            .chain((16..24).map(|logical| 1u64 << logical))
            .collect();
        let sets = cpu_sets(&core_masks, 8, 8);
        let topology = topology(&core_masks, &[0x00ff_ffff]);
        let allocation = build(&sets, &topology, Some(4), CpuMode::Balanced).unwrap();
        assert!(allocation.supported);
        assert_eq!(allocation.game_core_count, Some(4));
        assert_eq!(allocation.game_mask, 0xff);
        assert_eq!(allocation.game_mask & allocation.background_mask, 0);
        assert_eq!(
            allocation.game_mask | allocation.background_mask,
            allocation.all_mask
        );
    }

    #[test]
    fn l3_locality_is_preferred_for_smt_and_non_smt_cores() {
        let core_masks = [0b11, 0b1100, 0b11_0000, 0b1100_0000, 1 << 8, 1 << 9];
        let sets = cpu_sets(&core_masks, 6, 2);
        let topology = topology(&core_masks, &[0b1111, 0b11_1111_0000]);
        let allocation = build(&sets, &topology, Some(3), CpuMode::Balanced).unwrap();
        assert_eq!(allocation.game_mask, 0b01_1111_0000);
        assert_eq!(allocation.game_mask & allocation.background_mask, 0);
    }

    #[test]
    fn low_core_systems_keep_a_background_core() {
        let core_masks = [0b11, 0b1100, 0b11_0000, 0b1100_0000];
        let sets = cpu_sets(&core_masks, core_masks.len(), core_masks.len());
        let topology = topology(&core_masks, &[0xff]);
        let allocation = build(&sets, &topology, None, CpuMode::Balanced).unwrap();
        assert!(allocation.supported);
        assert_eq!(allocation.game_core_count, Some(3));
        assert_eq!(allocation.game_mask & allocation.background_mask, 0);
        assert_eq!(allocation.background_mask.count_ones(), 2);
    }

    #[test]
    fn manual_selection_uses_six_of_eight_performance_cores() {
        let core_masks: Vec<_> = (0..8).map(|core| 3u64 << (core * 2)).collect();
        let sets = cpu_sets(&core_masks, 8, 8);
        let topology = topology(&core_masks, &[0xffff]);
        let allocation = build(&sets, &topology, Some(6), CpuMode::Balanced).unwrap();
        assert_eq!(allocation.game_core_count, Some(6));
        assert_eq!(allocation.default_game_core_count, Some(4));
        assert_eq!(allocation.game_mask.count_ones(), 12);
        assert_eq!(allocation.background_mask.count_ones(), 4);
        let focused = build(&sets, &topology, None, CpuMode::Focus).unwrap();
        assert_eq!(focused.game_core_count, Some(6));
    }

    #[test]
    fn scheduling_class_prioritizes_without_removing_p_cores() {
        let core_masks: Vec<_> = (0..8).map(|core| 3u64 << (core * 2)).collect();
        let mut sets = cpu_sets(&core_masks, 8, 8);
        for set in &mut sets {
            set.scheduling_class = if set.core_index == 7 { 2 } else { 1 };
        }
        let topology = topology(&core_masks, &[0xffff]);
        let allocation = build(&sets, &topology, Some(6), CpuMode::Balanced).unwrap();
        assert_eq!(allocation.performance_core_count, Some(8));
        assert_eq!(allocation.game_core_count, Some(6));
        assert_ne!(allocation.game_mask & core_masks[7], 0);
    }

    #[test]
    fn hybrid_with_all_p_cores_shares_one_p_core_with_background() {
        let core_masks = [0b11, 0b1100, 1 << 4, 1 << 5, 1 << 6, 1 << 7];
        let sets = cpu_sets(&core_masks, 2, 2);
        let topology = topology(&core_masks, &[0xff]);
        let allocation = build(&sets, &topology, Some(2), CpuMode::Focus).unwrap();
        assert_eq!(allocation.game_core_count, Some(2));
        assert_eq!(allocation.game_mask, 0b1111);
        assert_eq!((allocation.game_mask & allocation.background_mask).count_ones(), 2);
        assert_eq!(allocation.background_mask.count_ones(), 6);
    }

    #[test]
    fn multiple_processor_groups_disable_splitting() {
        let topology = LogicalTopology {
            groups: vec![
                GroupMask { group: 0, mask: 3 },
                GroupMask { group: 1, mask: 3 },
            ],
            ..Default::default()
        };
        let allocation = build(&[], &topology, None, CpuMode::Balanced).unwrap();
        assert!(!allocation.supported);
        assert_eq!(allocation.processor_group_count, 2);
        assert!(
            allocation.topology_error.unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("processor group")
        );
    }

    #[test]
    fn malformed_api_buffers_are_rejected() {
        assert!(parse_cpu_sets(&[0; 8]).is_err());
        assert!(parse_relationships(&[0; 8], RelationProcessorCore).is_err());
        assert!(parse_groups(&[0; 8]).is_err());
    }

    #[test]
    fn live_windows_topology_returns_complete_usable_masks() {
        let allocation = resolve(None).unwrap();
        if allocation.supported {
            assert_ne!(allocation.game_mask, 0);
            assert_ne!(allocation.background_mask, 0);
            assert_eq!(
                allocation.game_mask | allocation.background_mask,
                allocation.all_mask
            );
        } else {
            assert!(allocation.topology_error.is_some());
        }
    }
}
