use crate::{Result, storage, workers::Environment};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::OpenOptions,
    io::Write,
    net::{IpAddr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const REMOTE_URL: &str = "https://raw.githubusercontent.com/rubystarashe/nogirem/main/channel.csv";
const MAX_CSV_BYTES: usize = 64 * 1024;
const SAMPLE_LIMIT: usize = 20;
const MEASURE_INTERVAL: Duration = Duration::from_secs(60);
const WARMUP_MEASURE_INTERVAL: Duration = Duration::from_secs(10);
const FAILURE_SYNC_INTERVAL: Duration = Duration::from_secs(60 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(900);
const ACTIVE_LATENCY_WINDOW: Duration = Duration::from_secs(3 * 60);
type Microseconds = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Endpoint {
    channel: u16,
    address: Option<SocketAddr>,
}

#[derive(Clone, Debug)]
struct Measurement {
    endpoint: Endpoint,
    outcomes: VecDeque<Option<Microseconds>>,
    measured_times: VecDeque<u64>,
    failed: bool,
    measured_at: Option<u64>,
}

#[derive(Debug, PartialEq)]
struct Quality {
    median_us: Option<Microseconds>,
    variation_score: Option<u32>,
    peak_variation_us: Option<Microseconds>,
    failure_percent: u32,
    score: Option<u32>,
    label: &'static str,
}

impl Measurement {
    fn record(&mut self, result: Option<Microseconds>, measured_at: u64) {
        self.failed = result.is_none();
        self.measured_at = Some(measured_at);
        self.outcomes.push_back(result);
        self.measured_times.push_back(measured_at);
        while self.outcomes.len() > SAMPLE_LIMIT {
            self.outcomes.pop_front();
            self.measured_times.pop_front();
        }
    }

    fn quality(&self) -> Quality {
        let mut successes = self
            .outcomes
            .iter()
            .filter_map(|value| *value)
            .collect::<Vec<_>>();
        successes.sort_unstable();
        let failures = self.outcomes.len().saturating_sub(successes.len());
        let failure_percent = if self.outcomes.is_empty() {
            0
        } else {
            (failures as u32 * 100 + self.outcomes.len() as u32 / 2) / self.outcomes.len() as u32
        };
        let Some(median_us) = median(&successes) else {
            return Quality {
                median_us: None,
                variation_score: None,
                peak_variation_us: None,
                failure_percent,
                score: None,
                label: if self.outcomes.is_empty() {
                    "확인 중"
                } else {
                    "연결 불가"
                },
            };
        };
        let sample_count = successes.len() as u64;
        let sample_sum = successes.iter().map(|value| u64::from(*value)).sum::<u64>();
        let scaled_deviations = successes
            .iter()
            .map(|value| (u64::from(*value) * sample_count).abs_diff(sample_sum))
            .collect::<Vec<_>>();
        let score_denominator = sample_count * sample_count * 10;
        let variation_score = ((scaled_deviations.iter().copied().sum::<u64>()
            + score_denominator / 2)
            / score_denominator) as u32;
        let peak_variation_us = scaled_deviations
            .iter()
            .copied()
            .max()
            .map(|value| ((value + sample_count / 2) / sample_count) as u32)
            .unwrap_or(0);
        let label = if self.outcomes.len() < 3 {
            "확인 중"
        } else if failure_percent >= 20 || variation_score >= 300 || peak_variation_us >= 10_000 {
            "불안정"
        } else if failure_percent > 0 || variation_score >= 200 || peak_variation_us >= 4_000 {
            "보통"
        } else {
            "안정적"
        };
        Quality {
            median_us: Some(median_us),
            variation_score: Some(variation_score),
            peak_variation_us: Some(peak_variation_us),
            failure_percent,
            score: (self.outcomes.len() >= 3).then_some(
                median_us
                    .saturating_add(variation_score.saturating_mul(20))
                    .saturating_add(peak_variation_us / 2)
                    .saturating_add(failure_percent.saturating_mul(200)),
            ),
            label,
        }
    }
}

fn median(sorted: &[Microseconds]) -> Option<Microseconds> {
    let middle = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        length if length % 2 == 0 => {
            Some((u64::from(sorted[middle - 1]) + u64::from(sorted[middle])).div_ceil(2) as u32)
        }
        _ => Some(sorted[middle]),
    }
}

fn milliseconds(microseconds: Option<Microseconds>) -> Option<f64> {
    microseconds.map(|value| f64::from(value) / 1_000.0)
}

fn hundredths_milliseconds(score: Option<u32>) -> Option<f64> {
    score.map(|value| f64::from(value) / 100.0)
}

fn measurement_span_ms(channels: &[Measurement]) -> Option<u64> {
    channels
        .iter()
        .filter_map(|entry| {
            entry
                .measured_times
                .back()?
                .checked_sub(*entry.measured_times.front()?)
        })
        .max()
}

#[derive(Debug)]
struct Runtime {
    channels: Vec<Measurement>,
    active_connection: Option<ActiveConnection>,
    active_latency_history: ActiveLatencyHistory,
    active_error: Option<String>,
    source: String,
    synced_at: Option<u64>,
    sync_error: Option<String>,
    input_error: Option<String>,
}

#[derive(Clone, Debug)]
struct ActiveConnection {
    identity: TcpConnectionIdentity,
    channel: u16,
    rtt_ms: Option<u32>,
    variation_ms: Option<u32>,
    estimated_latency_ms: Option<u32>,
    average_latency_ms: Option<u32>,
    maximum_latency_ms: Option<u32>,
    quality: &'static str,
    packets_retransmitted: u32,
    packets_retransmitted_delta: u32,
    timeouts: u32,
    timeouts_delta: u32,
    probe_error: Option<String>,
    measured_at: u64,
}

#[derive(Debug, Default)]
struct ActiveLatencyHistory {
    identity: Option<TcpConnectionIdentity>,
    samples: VecDeque<(Instant, u32)>,
    packets_retransmitted: Option<u32>,
    timeouts: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TcpConnectionIdentity {
    pid: u32,
    local_address: u32,
    local_port: u32,
    remote_address: u32,
    remote_port: u32,
}

pub struct ChannelPing {
    directory: PathBuf,
    enabled: AtomicBool,
    expanded: AtomicBool,
    runtime: RwLock<Runtime>,
    operation: Mutex<()>,
    active_checked_at: Mutex<Option<Instant>>,
}

impl ChannelPing {
    pub fn new(env: &Environment) -> Self {
        let directory = env.user.join("channel-ping");
        let cache = directory.join("channel.csv");
        let bundled = env.root.join("channel.csv");
        let (channels, source, sync_error) = load_initial(&cache, &bundled);
        let enabled = storage::read_json(&directory.join("setting.json"))
            .ok()
            .flatten()
            .and_then(|value| value["enabled"].as_bool())
            .unwrap_or(false);
        Self {
            directory,
            enabled: AtomicBool::new(enabled),
            expanded: AtomicBool::new(false),
            runtime: RwLock::new(Runtime {
                channels: measurements(channels, &[]),
                active_connection: None,
                active_latency_history: ActiveLatencyHistory::default(),
                active_error: None,
                source,
                synced_at: None,
                sync_error,
                input_error: None,
            }),
            operation: Mutex::new(()),
            active_checked_at: Mutex::new(None),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<Value> {
        storage::write_json(
            &self.directory.join("setting.json"),
            &json!({"enabled":enabled}),
        )?;
        self.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            self.expanded.store(false, Ordering::SeqCst);
            let mut runtime = self.runtime.write().unwrap();
            runtime.active_connection = None;
            runtime.active_latency_history = ActiveLatencyHistory::default();
            runtime.active_error = None;
        }
        Ok(self.status())
    }

    pub fn status(&self) -> Value {
        let runtime = self.runtime.read().unwrap();
        let best_score = runtime
            .channels
            .iter()
            .filter_map(|entry| entry.quality().score)
            .min();
        let measurement_span_ms = measurement_span_ms(&runtime.channels);
        json!({
            "enabled": self.enabled(),
            "expanded": self.expanded.load(Ordering::SeqCst),
            "source": runtime.source,
            "syncedAt": runtime.synced_at,
            "syncError": runtime.sync_error,
            "inputError": runtime.input_error,
            "activeConnectionError": runtime.active_error,
            "activeConnection": runtime.active_connection.as_ref().map(|active| json!({
                "channel": active.channel,
                "rttMs": active.rtt_ms,
                "variationMs": active.variation_ms,
                "estimatedLatencyMs": active.estimated_latency_ms,
                "averageLatencyMs": active.average_latency_ms,
                "maximumLatencyMs": active.maximum_latency_ms,
                "packetsRetransmitted": active.packets_retransmitted,
                "packetsRetransmittedDelta": active.packets_retransmitted_delta,
                "timeouts": active.timeouts,
                "timeoutsDelta": active.timeouts_delta,
                "quality": active.quality,
                "measuredAt": active.measured_at
            })),
            "measurementIntervalSeconds": MEASURE_INTERVAL.as_secs(),
            "warmupMeasurementIntervalSeconds": WARMUP_MEASURE_INTERVAL.as_secs(),
            "measurementSpanMs": measurement_span_ms,
            "channels": runtime.channels.iter().map(|entry| {
                let quality = entry.quality();
                json!({
                    "channel": entry.endpoint.channel,
                    "endpoint": entry.endpoint.address.map(|address| address.to_string()),
                    "averageMs": milliseconds(quality.median_us),
                    "medianMs": milliseconds(quality.median_us),
                    "variationMs": hundredths_milliseconds(quality.variation_score),
                    "variationScore": quality.variation_score,
                    "peakVariationMs": milliseconds(quality.peak_variation_us),
                    "failurePercent": quality.failure_percent,
                    "quality": quality.label,
                    "sampleCount": entry.outcomes.len(),
                    "failed": entry.failed,
                    "measuredAt": entry.measured_at,
                    "recommended": quality.score.zip(best_score)
                        .is_some_and(|(score, best)| score <= best.saturating_add(2_000))
                })
            }).collect::<Vec<_>>()
        })
    }

    pub fn set_input_error(&self, error: Option<String>) {
        self.runtime.write().unwrap().input_error = error;
    }

    pub fn set_expanded(&self, expanded: bool) {
        self.expanded.store(expanded, Ordering::SeqCst);
    }

    pub fn has_active_connection(&self) -> bool {
        self.runtime.read().unwrap().active_connection.is_some()
    }

    pub fn refresh_active_connection(&self, env: &Environment, game_pid: u32) {
        let Ok(_operation) = self.operation.try_lock() else {
            return;
        };
        let mut checked_at = self.active_checked_at.lock().unwrap();
        if checked_at.is_some_and(|checked_at| checked_at.elapsed() < Duration::from_millis(500)) {
            return;
        }
        *checked_at = Some(Instant::now());
        drop(checked_at);
        let (endpoints, previous_identity) = {
            let runtime = self.runtime.read().unwrap();
            (
                runtime
                    .channels
                    .iter()
                    .map(|entry| entry.endpoint.clone())
                    .collect::<Vec<_>>(),
                runtime
                    .active_connection
                    .as_ref()
                    .map(|connection| connection.identity),
            )
        };
        let result = active_game_connection(env, &endpoints, game_pid, previous_identity);
        let mut runtime = self.runtime.write().unwrap();
        let mut next = match result {
            Ok(next) => next,
            Err(error) => {
                runtime.active_connection = None;
                runtime.active_error = Some(error);
                return;
            }
        };
        if let Some(current) = next.as_mut()
            && current.rtt_ms.is_none()
            && let Some(label) = runtime
                .channels
                .iter()
                .find(|entry| entry.endpoint.channel == current.channel)
                .map(|entry| entry.quality().label)
                .filter(|label| matches!(*label, "안정적" | "보통" | "불안정"))
        {
            current.quality = label;
        }
        if let Some(current) = next.as_mut() {
            update_active_latency_history(&mut runtime.active_latency_history, current);
        } else {
            runtime.active_latency_history = ActiveLatencyHistory::default();
        }
        runtime.active_error = next
            .as_ref()
            .and_then(|connection| connection.probe_error.clone());
        runtime.active_connection = next;
    }

    pub fn run(&self, stopped: impl Fn() -> bool) {
        if let Err(error) = self.refresh_remote() {
            self.runtime.write().unwrap().sync_error = Some(error);
        }
        let mut next_measurement = Instant::now();
        let mut measurement_cursor = 0usize;
        let mut last_failure_sync = Some(Instant::now());
        while !stopped() {
            if self.enabled() && Instant::now() >= next_measurement {
                let (failed, count) = self.measure_next(measurement_cursor);
                measurement_cursor = measurement_cursor.saturating_add(1);
                next_measurement =
                    Instant::now() + measurement_spacing(count, self.is_warming_up());
                if failed
                    && last_failure_sync.is_none_or(|last| last.elapsed() >= FAILURE_SYNC_INTERVAL)
                {
                    last_failure_sync = Some(Instant::now());
                    if let Err(error) = self.refresh_remote() {
                        self.runtime.write().unwrap().sync_error = Some(error);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn is_warming_up(&self) -> bool {
        self.runtime
            .read()
            .unwrap()
            .channels
            .iter()
            .any(|entry| entry.endpoint.address.is_some() && entry.outcomes.len() < 3)
    }

    fn refresh_remote(&self) -> Result<()> {
        let _operation = self.operation.lock().unwrap();
        let response = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| error.to_string())?
            .get(REMOTE_URL)
            .header("User-Agent", "nogirem-channel-ping")
            .send()
            .map_err(|error| format!("채널 정보 다운로드 실패: {error}"))?
            .error_for_status()
            .map_err(|error| format!("채널 정보 응답 오류: {error}"))?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CSV_BYTES as u64)
        {
            return Err("채널 정보 파일이 허용 크기를 초과했습니다".into());
        }
        let bytes = response
            .bytes()
            .map_err(|error| format!("채널 정보 읽기 실패: {error}"))?;
        if bytes.len() > MAX_CSV_BYTES {
            return Err("채널 정보 파일이 허용 크기를 초과했습니다".into());
        }
        let text =
            std::str::from_utf8(&bytes).map_err(|_| "채널 정보 파일이 UTF-8 형식이 아닙니다")?;
        let endpoints = parse_csv(text)?;
        write_atomic(&self.directory.join("channel.csv"), text.as_bytes())?;
        let mut runtime = self.runtime.write().unwrap();
        runtime.channels = measurements(endpoints, &runtime.channels);
        runtime.source = "remote".into();
        runtime.synced_at = Some(crate::now_ms());
        runtime.sync_error = None;
        Ok(())
    }

    fn measure_next(&self, cursor: usize) -> (bool, usize) {
        let endpoints = self
            .runtime
            .read()
            .unwrap()
            .channels
            .iter()
            .filter(|entry| entry.endpoint.address.is_some())
            .map(|entry| entry.endpoint.clone())
            .collect::<Vec<_>>();
        let count = endpoints.len();
        let Some(endpoint) = endpoints.get(cursor % count.max(1)).cloned() else {
            return (false, 0);
        };
        let address = endpoint
            .address
            .expect("필터링된 채널에는 주소가 있어야 합니다");
        let started = Instant::now();
        let result = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .is_ok()
            .then(|| started.elapsed().as_micros())
            .map(|microseconds| microseconds.max(1).min(u128::from(u32::MAX)) as u32);
        let measured_at = crate::now_ms();
        let mut runtime = self.runtime.write().unwrap();
        if let Some(entry) = runtime
            .channels
            .iter_mut()
            .find(|entry| entry.endpoint == endpoint)
        {
            entry.record(result, measured_at);
        }
        (result.is_none(), count)
    }
}

fn measurement_spacing(count: usize, warming_up: bool) -> Duration {
    if count == 0 {
        return MEASURE_INTERVAL;
    }
    let interval = if warming_up {
        WARMUP_MEASURE_INTERVAL
    } else {
        MEASURE_INTERVAL
    };
    (interval / count as u32).max(Duration::from_millis(250))
}

#[cfg(windows)]
fn active_game_connection(
    env: &Environment,
    endpoints: &[Endpoint],
    game_pid: u32,
    previous_identity: Option<TcpConnectionIdentity>,
) -> Result<Option<ActiveConnection>> {
    use std::{mem::size_of, net::Ipv4Addr, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, GetPerTcpConnectionEStats, MIB_TCP_STATE_ESTAB, MIB_TCPROW_LH,
            MIB_TCPROW_LH_0, MIB_TCPROW_OWNER_PID, SetPerTcpConnectionEStats,
            TCP_ESTATS_PATH_ROD_v0, TCP_ESTATS_PATH_RW_v0, TCP_TABLE_OWNER_PID_CONNECTIONS,
            TcpConnectionEstatsPath,
        },
        Networking::WinSock::AF_INET,
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            QueryFullProcessImageNameW, WaitForSingleObject,
        },
    };

    struct ProcessHandle(HANDLE);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    let config = crate::workers::read(&env.root.join("config.json"));
    let raw_handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            game_pid,
        )
    };
    if raw_handle.is_null() {
        return Err(format!(
            "게임 프로세스 확인 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    let game_handle = ProcessHandle(raw_handle);
    let mut path_data = vec![0u16; 32768];
    let mut path_length = path_data.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(game_handle.0, 0, path_data.as_mut_ptr(), &mut path_length)
    } == 0
    {
        return Err(format!(
            "게임 실행 경로 확인 실패: {}",
            std::io::Error::last_os_error()
        ));
    }
    let path = String::from_utf16_lossy(&path_data[..path_length as usize]);
    let process = crate::process::Process {
        pid: game_pid,
        name: Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: Some(path),
        start_time: None,
        session_id: None,
    };
    if !crate::process::matches_game(&process, &config)
        || unsafe { WaitForSingleObject(game_handle.0, 0) } != 258
    {
        return Ok(None);
    }
    let endpoint_channels = endpoints
        .iter()
        .filter_map(|endpoint| endpoint.address.map(|address| (address, endpoint.channel)))
        .collect::<BTreeMap<_, _>>();

    let mut byte_size = 0u32;
    let first = unsafe {
        GetExtendedTcpTable(
            ptr::null_mut(),
            &mut byte_size,
            0,
            u32::from(AF_INET),
            TCP_TABLE_OWNER_PID_CONNECTIONS,
            0,
        )
    };
    if first != 122 || byte_size < size_of::<u32>() as u32 {
        return Err(format!("TCP 연결 목록 크기 확인 실패: {first}"));
    }
    let mut table = Vec::new();
    let mut result = 122;
    for _ in 0..2 {
        table = vec![0u32; (byte_size as usize).div_ceil(size_of::<u32>())];
        result = unsafe {
            GetExtendedTcpTable(
                table.as_mut_ptr().cast(),
                &mut byte_size,
                0,
                u32::from(AF_INET),
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            )
        };
        if result != 122 {
            break;
        }
    }
    if result != 0 {
        return Err(format!("TCP 연결 목록 조회 실패: {result}"));
    }
    let count = table[0] as usize;
    if !valid_tcp_table_size(byte_size as usize, count, size_of::<MIB_TCPROW_OWNER_PID>()) {
        return Err("TCP 연결 목록 크기가 올바르지 않습니다".into());
    }
    let rows = unsafe {
        std::slice::from_raw_parts(table.as_ptr().add(1).cast::<MIB_TCPROW_OWNER_PID>(), count)
    };
    let mut candidates = Vec::new();
    for owner in rows {
        if owner.dwState != MIB_TCP_STATE_ESTAB as u32 || owner.dwOwningPid != game_pid {
            continue;
        }
        let address = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from(mib_ipv4_octets(owner.dwRemoteAddr))),
            mib_port(owner.dwRemotePort),
        );
        let Some(&channel) = endpoint_channels.get(&address) else {
            continue;
        };
        let row = MIB_TCPROW_LH {
            Anonymous: MIB_TCPROW_LH_0 {
                dwState: owner.dwState,
            },
            dwLocalAddr: owner.dwLocalAddr,
            dwLocalPort: owner.dwLocalPort,
            dwRemoteAddr: owner.dwRemoteAddr,
            dwRemotePort: owner.dwRemotePort,
        };
        let mut enabled = TCP_ESTATS_PATH_RW_v0 {
            EnableCollection: false,
        };
        let mut path = unsafe { std::mem::zeroed::<TCP_ESTATS_PATH_ROD_v0>() };
        let read_result = unsafe {
            GetPerTcpConnectionEStats(
                &row,
                TcpConnectionEstatsPath,
                (&mut enabled as *mut TCP_ESTATS_PATH_RW_v0).cast(),
                0,
                size_of::<TCP_ESTATS_PATH_RW_v0>() as u32,
                ptr::null_mut(),
                0,
                0,
                (&mut path as *mut TCP_ESTATS_PATH_ROD_v0).cast(),
                0,
                size_of::<TCP_ESTATS_PATH_ROD_v0>() as u32,
            )
        };
        let identity = TcpConnectionIdentity {
            pid: game_pid,
            local_address: owner.dwLocalAddr,
            local_port: owner.dwLocalPort,
            remote_address: owner.dwRemoteAddr,
            remote_port: owner.dwRemotePort,
        };
        if read_result == 0 && enabled.EnableCollection && path.CountRtt > 0 {
            let rtt = path.SmoothedRtt.max(1);
            let variation = path.RttVar;
            let quality = if variation >= 4u32.max(rtt / 2) {
                "불안정"
            } else if variation >= 2u32.max(rtt / 4) {
                "보통"
            } else {
                "안정적"
            };
            candidates.push(ActiveConnection {
                identity,
                channel,
                rtt_ms: Some(rtt),
                variation_ms: Some(variation),
                estimated_latency_ms: Some(estimate_latency(rtt, variation, false)),
                average_latency_ms: None,
                maximum_latency_ms: None,
                quality,
                packets_retransmitted: path.PktsRetrans,
                packets_retransmitted_delta: 0,
                timeouts: path.Timeouts,
                timeouts_delta: 0,
                probe_error: None,
                measured_at: crate::now_ms(),
            });
            continue;
        }
        let enable = TCP_ESTATS_PATH_RW_v0 {
            EnableCollection: true,
        };
        let enable_result = unsafe {
            SetPerTcpConnectionEStats(
                &row,
                TcpConnectionEstatsPath,
                (&enable as *const TCP_ESTATS_PATH_RW_v0).cast(),
                0,
                size_of::<TCP_ESTATS_PATH_RW_v0>() as u32,
                0,
            )
        };
        let probe_error = if read_result != 0 {
            Some(format!("실시간 RTT 조회 준비 실패: {read_result}"))
        } else if enable_result != 0 {
            Some(format!("실시간 RTT 수집 활성화 실패: {enable_result}"))
        } else {
            None
        };
        candidates.push(ActiveConnection {
            identity,
            channel,
            rtt_ms: None,
            variation_ms: None,
            estimated_latency_ms: None,
            average_latency_ms: None,
            maximum_latency_ms: None,
            quality: "확인 중",
            packets_retransmitted: 0,
            packets_retransmitted_delta: 0,
            timeouts: 0,
            timeouts_delta: 0,
            probe_error,
            measured_at: crate::now_ms(),
        });
    }
    if unsafe { WaitForSingleObject(game_handle.0, 0) } != 258 {
        return Ok(None);
    }
    select_active_candidate(candidates, previous_identity)
}

#[cfg(not(windows))]
fn active_game_connection(
    _env: &Environment,
    _endpoints: &[Endpoint],
    _game_pid: u32,
    _previous_identity: Option<TcpConnectionIdentity>,
) -> Result<Option<ActiveConnection>> {
    Ok(None)
}

fn mib_ipv4_octets(value: u32) -> [u8; 4] {
    value.to_ne_bytes()
}

fn mib_port(value: u32) -> u16 {
    let bytes = value.to_ne_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn valid_tcp_table_size(byte_size: usize, count: usize, row_size: usize) -> bool {
    size_of_u32()
        .checked_add(count.checked_mul(row_size).unwrap_or(usize::MAX))
        .is_some_and(|required| required <= byte_size)
}

const fn size_of_u32() -> usize {
    std::mem::size_of::<u32>()
}

fn select_active_candidate(
    mut candidates: Vec<ActiveConnection>,
    previous_identity: Option<TcpConnectionIdentity>,
) -> Result<Option<ActiveConnection>> {
    if let Some(previous_identity) = previous_identity
        && let Some(index) = candidates
            .iter()
            .position(|candidate| candidate.identity == previous_identity)
    {
        return Ok(Some(candidates.swap_remove(index)));
    }
    if candidates.len() <= 1 {
        return Ok(candidates.pop());
    }
    let mut with_rtt = candidates
        .into_iter()
        .filter(|candidate| candidate.rtt_ms.is_some());
    let first = with_rtt.next();
    if first.is_some() && with_rtt.next().is_none() {
        return Ok(first);
    }
    Err("현재 채널 연결이 여러 개라 안전하게 구분하지 못했습니다".into())
}

fn estimate_latency(rtt_ms: u32, variation_ms: u32, degraded: bool) -> u32 {
    rtt_ms.saturating_add(variation_ms.saturating_mul(if degraded { 4 } else { 1 }))
}

fn update_active_latency_history(
    history: &mut ActiveLatencyHistory,
    current: &mut ActiveConnection,
) {
    if history.identity != Some(current.identity) {
        history.identity = Some(current.identity);
        history.samples.clear();
        history.packets_retransmitted = None;
        history.timeouts = None;
    }
    let now = Instant::now();
    while history
        .samples
        .front()
        .is_some_and(|(measured_at, _)| now.duration_since(*measured_at) > ACTIVE_LATENCY_WINDOW)
    {
        history.samples.pop_front();
    }
    if let (Some(rtt_ms), Some(variation_ms)) = (current.rtt_ms, current.variation_ms) {
        current.packets_retransmitted_delta = history
            .packets_retransmitted
            .map(|previous| current.packets_retransmitted.saturating_sub(previous))
            .unwrap_or(0);
        current.timeouts_delta = history
            .timeouts
            .map(|previous| current.timeouts.saturating_sub(previous))
            .unwrap_or(0);
        history.packets_retransmitted = Some(current.packets_retransmitted);
        history.timeouts = Some(current.timeouts);
        let degraded = current.packets_retransmitted_delta > 0 || current.timeouts_delta > 0;
        let latency_ms = estimate_latency(rtt_ms, variation_ms, degraded);
        current.estimated_latency_ms = Some(latency_ms);
        if degraded {
            current.quality = "불안정";
        }
        history.samples.push_back((now, latency_ms));
    }
    if history.samples.is_empty() {
        current.average_latency_ms = None;
        current.maximum_latency_ms = None;
        return;
    }
    let sum = history
        .samples
        .iter()
        .map(|(_, value)| u64::from(*value))
        .sum::<u64>();
    current.average_latency_ms =
        Some(((sum + history.samples.len() as u64 / 2) / history.samples.len() as u64) as u32);
    current.maximum_latency_ms = history.samples.iter().map(|(_, value)| *value).max();
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputDecision {
    None,
    Show(OverlayBounds, u32),
    Hide,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InputEvent {
    Key {
        code: usize,
        down: bool,
        foreground: usize,
    },
    MouseDown,
}

static INPUT_EVENTS: Mutex<Option<SyncSender<InputEvent>>> = Mutex::new(None);
static INPUT_KEYS: Mutex<[bool; 256]> = Mutex::new([false; 256]);
static INPUT_OVERFLOW: AtomicBool = AtomicBool::new(false);
static INPUT_OVERLAY_WINDOW: Mutex<Option<(u64, isize)>> = Mutex::new(None);

pub fn set_input_overlay_window(window: Option<(u64, isize)>) {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock() {
        *registered = window;
    }
}

pub fn clear_input_overlay_window(window_id: u64) {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock()
        && registered.is_some_and(|(id, _)| id == window_id)
    {
        *registered = None;
    }
}

#[cfg(windows)]
fn hide_input_overlay() {
    if let Ok(mut registered) = INPUT_OVERLAY_WINDOW.lock()
        && let Some((_, hwnd)) = registered.take()
    {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindowAsync(
                hwnd as _,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
            );
        }
    }
}

pub struct InputMonitor {
    ignored: [bool; 256],
    events: Receiver<InputEvent>,
    blocked_until: Option<Instant>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl InputMonitor {
    pub fn new() -> Result<Self> {
        let (events, receiver) = sync_channel(256);
        let (ready, started) = sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let hook_stopping = stopping.clone();
        let thread = std::thread::spawn(move || input_hook_loop(events, hook_stopping, ready));
        let start = started.recv_timeout(Duration::from_secs(3));
        let start = match start {
            Ok(start) => start,
            Err(_) => {
                stopping.store(true, Ordering::Release);
                let _ = thread.join();
                return Err("입력 감시 시작 응답이 없습니다".into());
            }
        };
        if let Err(error) = start {
            stopping.store(true, Ordering::Release);
            let _ = thread.join();
            return Err(error);
        }
        Ok(Self {
            ignored: [false; 256],
            events: receiver,
            blocked_until: None,
            stopping,
            thread: Some(thread),
        })
    }

    pub fn poll(&mut self, visible: bool, env: &Environment) -> InputDecision {
        if take_input_overflow(&mut self.ignored) {
            self.blocked_until = Some(Instant::now() + Duration::from_millis(100));
            while self.events.try_recv().is_ok() {}
            return InputDecision::Hide;
        }
        if let Some(until) = self.blocked_until {
            let mut drained = false;
            while self.events.try_recv().is_ok() {
                drained = true;
            }
            if drained {
                self.blocked_until = Some(Instant::now() + Duration::from_millis(100));
                return if visible {
                    InputDecision::Hide
                } else {
                    InputDecision::None
                };
            }
            if Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
                return if visible {
                    InputDecision::Hide
                } else {
                    InputDecision::None
                };
            }
            self.blocked_until = None;
        }
        let event = match self.events.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return InputDecision::None,
            Err(RecvTimeoutError::Disconnected) => return InputDecision::Unavailable,
        };
        let target = match event {
            InputEvent::Key {
                code: 0x5b | 0x5c,
                down: true,
                foreground,
            } if !visible => game_bounds(env, foreground),
            _ => None,
        };
        decide_event(visible, event, &mut self.ignored, target)
    }
}

fn take_input_overflow(ignored: &mut [bool; 256]) -> bool {
    if !INPUT_OVERFLOW.swap(false, Ordering::AcqRel) {
        return false;
    }
    ignored.fill(false);
    true
}

impl Drop for InputMonitor {
    fn drop(&mut self) {
        set_input_overlay_window(None);
        self.stopping.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn decide_event(
    visible: bool,
    event: InputEvent,
    ignored: &mut [bool; 256],
    game_target: Option<(OverlayBounds, u32)>,
) -> InputDecision {
    match event {
        InputEvent::Key {
            code, down: false, ..
        } => {
            if code < ignored.len() {
                ignored[code] = false;
            }
            InputDecision::None
        }
        InputEvent::Key {
            code, down: true, ..
        } if visible => {
            if code < ignored.len() && ignored[code] {
                InputDecision::None
            } else {
                InputDecision::Hide
            }
        }
        InputEvent::MouseDown if visible => InputDecision::Hide,
        InputEvent::Key {
            code: code @ (0x5b | 0x5c),
            down: true,
            ..
        } => game_target.map_or(InputDecision::None, |(bounds, game_pid)| {
            ignored[code] = true;
            InputDecision::Show(bounds, game_pid)
        }),
        _ => InputDecision::None,
    }
}

#[cfg(windows)]
unsafe extern "system" fn keyboard_hook(
    code: i32,
    parameter: windows_sys::Win32::Foundation::WPARAM,
    data: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };
    if code == HC_ACTION as i32 {
        let input = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
        let down = parameter == WM_KEYDOWN as usize || parameter == WM_SYSKEYDOWN as usize;
        let up = parameter == WM_KEYUP as usize || parameter == WM_SYSKEYUP as usize;
        if down || up {
            let key = input.vkCode as usize;
            let edge = INPUT_KEYS.lock().ok().is_some_and(|mut keys| {
                if key >= keys.len() || keys[key] == down {
                    return false;
                }
                keys[key] = down;
                true
            });
            if edge {
                if down {
                    hide_input_overlay();
                }
                send_input_event(InputEvent::Key {
                    code: key,
                    down,
                    foreground: if down {
                        (unsafe { GetForegroundWindow() }) as usize
                    } else {
                        0
                    },
                });
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, parameter, data) }
}

#[cfg(windows)]
unsafe extern "system" fn mouse_hook(
    code: i32,
    parameter: windows_sys::Win32::Foundation::WPARAM,
    data: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
    };
    if code == HC_ACTION as i32
        && matches!(
            parameter as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        hide_input_overlay();
        send_input_event(InputEvent::MouseDown);
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, parameter, data) }
}

#[cfg(windows)]
fn send_input_event(event: InputEvent) {
    if let Ok(sender) = INPUT_EVENTS.lock()
        && let Some(sender) = sender.as_ref()
    {
        if matches!(sender.try_send(event), Err(TrySendError::Full(_))) {
            INPUT_OVERFLOW.store(true, Ordering::Release);
        }
    }
}

#[cfg(windows)]
fn input_hook_loop(
    events: SyncSender<InputEvent>,
    stopping: Arc<AtomicBool>,
    ready: SyncSender<Result<()>>,
) {
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
            PeekMessageW, QS_ALLINPUT, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
            WH_KEYBOARD_LL, WH_MOUSE_LL,
        },
    };
    *INPUT_EVENTS.lock().unwrap() = Some(events);
    INPUT_KEYS.lock().unwrap().fill(false);
    INPUT_OVERFLOW.store(false, Ordering::Release);
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) };
    let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
    if keyboard.is_null() || mouse.is_null() {
        if !keyboard.is_null() {
            unsafe {
                UnhookWindowsHookEx(keyboard);
            }
        }
        if !mouse.is_null() {
            unsafe {
                UnhookWindowsHookEx(mouse);
            }
        }
        *INPUT_EVENTS.lock().unwrap() = None;
        let _ = ready.send(Err("키보드·마우스 입력 감시를 시작하지 못했습니다".into()));
        return;
    }
    let _ = ready.send(Ok(()));
    let mut message = unsafe { std::mem::zeroed::<MSG>() };
    while !stopping.load(Ordering::Acquire) {
        while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        unsafe {
            MsgWaitForMultipleObjectsEx(0, std::ptr::null(), 50, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }
    unsafe {
        UnhookWindowsHookEx(keyboard);
        UnhookWindowsHookEx(mouse);
    }
    *INPUT_EVENTS.lock().unwrap() = None;
    INPUT_KEYS.lock().unwrap().fill(false);
    INPUT_OVERFLOW.store(false, Ordering::Release);
}

#[cfg(windows)]
fn game_bounds(env: &Environment, window: usize) -> Option<(OverlayBounds, u32)> {
    overlay_bounds(env, window, 584.0, 400.0, None)
}

#[cfg(windows)]
pub fn foreground_compact_target(env: &Environment) -> Option<(OverlayBounds, u32)> {
    let window =
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } as usize;
    overlay_bounds(env, window, 420.0, 30.0, Some(12.0))
}

#[cfg(windows)]
pub fn foreground_is_windows_shell() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return false;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let Ok(path) = crate::process::path(pid) else {
        return false;
    };
    let Some(name) = Path::new(&path).file_name() else {
        return false;
    };
    matches!(
        name.to_string_lossy().to_ascii_lowercase().as_str(),
        "startmenuexperiencehost.exe"
            | "shellexperiencehost.exe"
            | "searchhost.exe"
            | "searchapp.exe"
    )
}

#[cfg(windows)]
fn overlay_bounds(
    env: &Environment,
    window: usize,
    width: f64,
    height: f64,
    top_offset: Option<f64>,
) -> Option<(OverlayBounds, u32)> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::HiDpi::GetDpiForWindow,
        UI::WindowsAndMessaging::{GetClientRect, GetWindowThreadProcessId},
    };
    let window = window as windows_sys::Win32::Foundation::HWND;
    if window.is_null() {
        return None;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let path = crate::process::path(pid).ok()?;
    let process = crate::process::Process {
        pid,
        name: Path::new(&path).file_name()?.to_string_lossy().into_owned(),
        path: Some(path),
        start_time: None,
        session_id: None,
    };
    let config = crate::workers::read(&env.root.join("config.json"));
    if !crate::process::matches_game(&process, &config) {
        return None;
    }
    let mut client = RECT::default();
    if unsafe { GetClientRect(window, &mut client) } == 0 {
        return None;
    }
    let mut origin = POINT {
        x: client.left,
        y: client.top,
    };
    if unsafe { ClientToScreen(window, &mut origin) } == 0 {
        return None;
    }
    let scale = f64::from(unsafe { GetDpiForWindow(window) }.max(96)) / 96.0;
    let physical_width = width * scale;
    let physical_height = height * scale;
    let client_width = f64::from(client.right - client.left);
    let client_height = f64::from(client.bottom - client.top);
    Some((
        OverlayBounds {
            x: f64::from(origin.x) + (client_width - physical_width) / 2.0,
            y: top_offset.map_or_else(
                || f64::from(origin.y) + (client_height - physical_height) / 2.0,
                |offset| f64::from(origin.y) + offset * scale,
            ),
            width: physical_width,
            height: physical_height,
        },
        pid,
    ))
}

#[cfg(not(windows))]
fn game_bounds(_env: &Environment, _window: usize) -> Option<(OverlayBounds, u32)> {
    None
}

#[cfg(not(windows))]
pub fn foreground_compact_target(_env: &Environment) -> Option<(OverlayBounds, u32)> {
    None
}

#[cfg(not(windows))]
pub fn foreground_is_windows_shell() -> bool {
    false
}

#[cfg(not(windows))]
fn input_hook_loop(
    _events: SyncSender<InputEvent>,
    _stopping: Arc<AtomicBool>,
    ready: SyncSender<Result<()>>,
) {
    let _ = ready.send(Err("Windows에서만 입력 감시를 사용할 수 있습니다".into()));
}

fn load_initial(cache: &Path, bundled: &Path) -> (Vec<Endpoint>, String, Option<String>) {
    let mut errors = Vec::new();
    for (path, source) in [(cache, "cache"), (bundled, "bundled")] {
        match std::fs::read_to_string(path) {
            Ok(text) => match parse_csv(&text) {
                Ok(channels) => return (channels, source.into(), None),
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    (
        Vec::new(),
        "unavailable".into(),
        (!errors.is_empty()).then(|| errors.join("; ")),
    )
}

fn measurements(endpoints: Vec<Endpoint>, previous: &[Measurement]) -> Vec<Measurement> {
    endpoints
        .into_iter()
        .map(|endpoint| {
            previous
                .iter()
                .find(|entry| entry.endpoint == endpoint)
                .cloned()
                .unwrap_or(Measurement {
                    endpoint,
                    outcomes: VecDeque::new(),
                    measured_times: VecDeque::new(),
                    failed: false,
                    measured_at: None,
                })
        })
        .collect()
}

fn parse_csv(text: &str) -> Result<Vec<Endpoint>> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .map(|line| line.trim_start_matches('\u{feff}').trim())
        .ok_or("채널 정보 파일이 비어 있습니다")?;
    if header != "채널,IP:포트" {
        return Err("채널 정보 헤더가 올바르지 않습니다".into());
    }
    let mut channels = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (channel, endpoint) = line
            .split_once(',')
            .ok_or_else(|| format!("{}행의 채널 정보가 올바르지 않습니다", index + 2))?;
        let channel = channel
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|channel| (1..=100).contains(channel))
            .ok_or_else(|| format!("{}행의 채널 번호가 올바르지 않습니다", index + 2))?;
        let address = if endpoint.trim().is_empty() {
            None
        } else {
            let (ip, port) = endpoint
                .trim()
                .rsplit_once(':')
                .ok_or_else(|| format!("{}행의 IP와 포트가 올바르지 않습니다", index + 2))?;
            let ip = ip
                .parse::<std::net::Ipv4Addr>()
                .map(IpAddr::V4)
                .map_err(|_| format!("{}행의 IPv4 주소가 올바르지 않습니다", index + 2))?;
            if !public_unicast(ip) {
                return Err(format!(
                    "{}행의 IP는 공개 unicast 주소가 아닙니다",
                    index + 2
                ));
            }
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| format!("{}행의 포트가 올바르지 않습니다", index + 2))?;
            Some(SocketAddr::new(ip, port))
        };
        if channels
            .insert(channel, Endpoint { channel, address })
            .is_some()
        {
            return Err(format!("{channel}채널이 중복되었습니다"));
        }
    }
    let maximum = channels.keys().next_back().copied().unwrap_or(0);
    if maximum == 0 || channels.len() != usize::from(maximum) {
        return Err("채널 번호는 1부터 빠짐없이 포함되어야 합니다".into());
    }
    Ok(channels.into_values().collect())
}

fn public_unicast(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            a != 0
                && a != 10
                && a != 127
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 169 && b == 254)
                && !(a == 172 && (16..=31).contains(&b))
                && !(a == 192 && b == 168)
                && !(a == 192 && b == 0 && c == 2)
                && !(a == 198 && (b == 18 || b == 19 || b == 51 && c == 100))
                && !(a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                return public_unicast(IpAddr::V4(ip));
            }
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && ip.segments()[0..2] != [0x2001, 0x0db8]
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or("채널 정보 캐시 경로가 올바르지 않습니다")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".channel.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    static INPUT_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn 초기_세_표본은_빠르게_순차_측정한다() {
        let warmup = measurement_spacing(38, true);
        assert!(warmup >= Duration::from_millis(250));
        assert!(warmup < Duration::from_millis(300));
        assert!(measurement_spacing(38, false) > warmup);
        assert_eq!(measurement_spacing(0, true), MEASURE_INTERVAL);
    }

    #[test]
    fn csv는_빈_endpoint와_주소를_검증한다() {
        let channels = parse_csv(
            "\u{feff}채널,IP:포트\r\n1,211.218.233.210:11020\r\n2,\r\n3,211.218.233.211:11021\r\n",
        )
        .unwrap();
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].channel, 1);
        assert!(channels[1].address.is_none());
        assert_eq!(channels[2].address.unwrap().port(), 11021);
        assert!(parse_csv("채널,IP:포트\n1,127.0.0.1:80\n").is_err());
        assert!(parse_csv("채널,IP:포트\n1,::ffff:127.0.0.1:80\n").is_err());
    }

    #[test]
    fn csv는_중복과_누락_채널을_거부한다() {
        assert!(parse_csv("채널,IP:포트\n1,\n1,\n").is_err());
        assert!(parse_csv("채널,IP:포트\n1,\n3,\n").is_err());
    }

    #[test]
    fn endpoint가_같으면_기존_표본을_보존한다() {
        let endpoint = Endpoint {
            channel: 1,
            address: Some("211.218.233.210:11020".parse().unwrap()),
        };
        let previous = vec![Measurement {
            endpoint: endpoint.clone(),
            outcomes: VecDeque::from([Some(10), Some(20)]),
            measured_times: VecDeque::from([1, 2]),
            failed: false,
            measured_at: Some(1),
        }];
        let retained = measurements(vec![endpoint.clone()], &previous);
        assert_eq!(retained[0].outcomes, VecDeque::from([Some(10), Some(20)]));
        assert_eq!(measurement_span_ms(&retained), Some(1));
        let changed = measurements(
            vec![Endpoint {
                channel: 1,
                address: Some("211.218.233.210:11021".parse().unwrap()),
            }],
            &previous,
        );
        assert!(changed[0].outcomes.is_empty());
        assert_eq!(measurement_span_ms(&changed), None);
    }

    #[test]
    fn 최근_성공과_실패로_안정성을_판정한다() {
        let mut measurement = Measurement {
            endpoint: Endpoint {
                channel: 1,
                address: None,
            },
            outcomes: VecDeque::new(),
            measured_times: VecDeque::new(),
            failed: false,
            measured_at: None,
        };
        for value in [
            Some(5_000),
            Some(5_000),
            Some(6_000),
            Some(5_000),
            Some(6_000),
        ] {
            measurement.record(value, 1);
        }
        assert_eq!(
            measurement.quality(),
            Quality {
                median_us: Some(5_000),
                variation_score: Some(48),
                peak_variation_us: Some(600),
                failure_percent: 0,
                score: Some(6_260),
                label: "안정적",
            }
        );
        measurement.record(None, 2);
        assert_eq!(measurement.quality().label, "보통");
        measurement.record(None, 3);
        assert_eq!(measurement.quality().label, "불안정");
    }

    #[test]
    fn 평균_절대편차_점수는_최종_단위에서_한번만_반올림한다() {
        let measurement = Measurement {
            endpoint: Endpoint {
                channel: 1,
                address: None,
            },
            outcomes: VecDeque::from([Some(1), Some(10)]),
            measured_times: VecDeque::from([1, 2]),
            failed: false,
            measured_at: Some(2),
        };
        assert_eq!(measurement.quality().variation_score, Some(0));
    }

    #[test]
    fn 드문_큰_지연도_안정적으로_표시하지_않는다() {
        let measurement = Measurement {
            endpoint: Endpoint {
                channel: 1,
                address: None,
            },
            outcomes: VecDeque::from([
                Some(5_000),
                Some(5_000),
                Some(5_000),
                Some(5_000),
                Some(30_000),
            ]),
            measured_times: VecDeque::from([1, 2, 3, 4, 5]),
            failed: false,
            measured_at: Some(1),
        };
        assert_eq!(measurement.quality().label, "불안정");
    }

    #[test]
    fn 성공_기록이_없으면_연결_불가로_표시한다() {
        let mut measurement = Measurement {
            endpoint: Endpoint {
                channel: 1,
                address: None,
            },
            outcomes: VecDeque::new(),
            measured_times: VecDeque::new(),
            failed: false,
            measured_at: None,
        };
        measurement.record(None, 1);
        assert_eq!(measurement.quality().label, "연결 불가");
        assert_eq!(measurement.quality().median_us, None);
    }

    fn active_candidate(
        identity: TcpConnectionIdentity,
        channel: u16,
        rtt_ms: Option<u32>,
    ) -> ActiveConnection {
        ActiveConnection {
            identity,
            channel,
            rtt_ms,
            variation_ms: rtt_ms.map(|_| 1),
            estimated_latency_ms: rtt_ms,
            average_latency_ms: None,
            maximum_latency_ms: None,
            quality: "안정적",
            packets_retransmitted: 0,
            packets_retransmitted_delta: 0,
            timeouts: 0,
            timeouts_delta: 0,
            probe_error: None,
            measured_at: 1,
        }
    }

    #[test]
    fn tcp_지연_추정치는_최근_창의_평균과_최대를_계산하고_연결별로_초기화한다() {
        let first = TcpConnectionIdentity {
            pid: 1,
            local_address: 1,
            local_port: 1,
            remote_address: 1,
            remote_port: 1,
        };
        let mut history = ActiveLatencyHistory::default();
        let mut current = active_candidate(first, 1, Some(0));
        for rtt in [10, 20, 30, 40] {
            current.rtt_ms = Some(rtt);
            current.estimated_latency_ms = Some(rtt);
            update_active_latency_history(&mut history, &mut current);
        }
        assert_eq!(current.average_latency_ms, Some(26));
        assert_eq!(current.maximum_latency_ms, Some(41));

        let second = TcpConnectionIdentity {
            remote_port: 2,
            ..first
        };
        let mut replaced = active_candidate(second, 2, Some(5));
        update_active_latency_history(&mut history, &mut replaced);
        assert_eq!(replaced.average_latency_ms, Some(6));
        assert_eq!(replaced.maximum_latency_ms, Some(6));
        assert_eq!(history.samples.len(), 1);
    }

    #[test]
    fn windows_tcp_주소와_port의_network_byte_order를_변환한다() {
        assert_eq!(
            mib_ipv4_octets(u32::from_ne_bytes([211, 218, 233, 210])),
            [211, 218, 233, 210]
        );
        assert_eq!(mib_port(u32::from_ne_bytes([0x2b, 0x0c, 0, 0])), 11020);
    }

    #[test]
    fn tcp_table_행_개수는_실제_buffer_크기를_넘을_수_없다() {
        let row_size = std::mem::size_of::<[u32; 6]>();
        assert!(valid_tcp_table_size(4 + row_size * 2, 2, row_size));
        assert!(!valid_tcp_table_size(4 + row_size, 2, row_size));
        assert!(!valid_tcp_table_size(usize::MAX, usize::MAX, row_size));
    }

    #[test]
    fn 복수_연결은_기존_5tuple을_유지하고_모호하면_표시하지_않는다() {
        let first = TcpConnectionIdentity {
            pid: 1,
            local_address: 1,
            local_port: 2,
            remote_address: 3,
            remote_port: 4,
        };
        let second = TcpConnectionIdentity {
            remote_port: 5,
            ..first
        };
        let selected = select_active_candidate(
            vec![
                active_candidate(first, 1, Some(5)),
                active_candidate(second, 2, Some(6)),
            ],
            Some(second),
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.identity, second);
        assert!(
            select_active_candidate(
                vec![
                    active_candidate(first, 1, Some(5)),
                    active_candidate(second, 2, Some(6)),
                ],
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn 재전송_증가는_같은_5tuple에서만_불안정으로_반영한다() {
        let identity = TcpConnectionIdentity {
            pid: 1,
            local_address: 1,
            local_port: 2,
            remote_address: 3,
            remote_port: 4,
        };
        let mut history = ActiveLatencyHistory::default();
        let mut previous = active_candidate(identity, 1, Some(5));
        update_active_latency_history(&mut history, &mut previous);
        let mut same = active_candidate(identity, 1, Some(5));
        same.packets_retransmitted = 1;
        update_active_latency_history(&mut history, &mut same);
        assert_eq!(same.quality, "불안정");
        assert_eq!(same.packets_retransmitted_delta, 1);
        assert_eq!(same.estimated_latency_ms, Some(9));

        let mut replaced = active_candidate(
            TcpConnectionIdentity {
                local_port: 9,
                ..identity
            },
            1,
            Some(5),
        );
        replaced.packets_retransmitted = 1;
        update_active_latency_history(&mut history, &mut replaced);
        assert_eq!(replaced.quality, "안정적");
        assert_eq!(replaced.packets_retransmitted_delta, 0);
    }

    #[test]
    fn 일시적_무효_표본은_최근_유효_counter_기준을_끊지_않는다() {
        let identity = TcpConnectionIdentity {
            pid: 1,
            local_address: 1,
            local_port: 2,
            remote_address: 3,
            remote_port: 4,
        };
        let mut history = ActiveLatencyHistory::default();
        let mut first = active_candidate(identity, 1, Some(5));
        first.packets_retransmitted = 10;
        first.timeouts = 2;
        update_active_latency_history(&mut history, &mut first);

        let mut invalid = active_candidate(identity, 1, None);
        update_active_latency_history(&mut history, &mut invalid);

        let mut recovered = active_candidate(identity, 1, Some(5));
        recovered.packets_retransmitted = 12;
        recovered.timeouts = 3;
        update_active_latency_history(&mut history, &mut recovered);
        assert_eq!(recovered.packets_retransmitted_delta, 2);
        assert_eq!(recovered.timeouts_delta, 1);
        assert_eq!(recovered.estimated_latency_ms, Some(9));
    }

    #[test]
    fn tcp_counter_감소는_reset으로_처리하고_다음_증가부터_반영한다() {
        let identity = TcpConnectionIdentity {
            pid: 1,
            local_address: 1,
            local_port: 2,
            remote_address: 3,
            remote_port: 4,
        };
        let mut history = ActiveLatencyHistory::default();
        let mut first = active_candidate(identity, 1, Some(5));
        first.packets_retransmitted = 10;
        update_active_latency_history(&mut history, &mut first);

        let mut reset = active_candidate(identity, 1, Some(5));
        reset.packets_retransmitted = 3;
        update_active_latency_history(&mut history, &mut reset);
        assert_eq!(reset.packets_retransmitted_delta, 0);

        let mut increased = active_candidate(identity, 1, Some(5));
        increased.packets_retransmitted = 4;
        update_active_latency_history(&mut history, &mut increased);
        assert_eq!(increased.packets_retransmitted_delta, 1);
        assert_eq!(increased.estimated_latency_ms, Some(9));
    }

    #[test]
    fn tcp_지연은_변동을_더하고_손실시_rto_계수로_추정한다() {
        assert_eq!(estimate_latency(10, 3, false), 13);
        assert_eq!(estimate_latency(10, 3, true), 22);
        assert_eq!(estimate_latency(u32::MAX, 10, true), u32::MAX);
    }

    #[test]
    fn 표시_trigger는_release까지_무시하고_새_down은_숨긴다() {
        let mut ignored = [false; 256];
        let bounds = OverlayBounds {
            x: 0.0,
            y: 0.0,
            width: 584.0,
            height: 400.0,
        };
        assert_eq!(
            decide_event(
                false,
                InputEvent::Key {
                    code: 0x5b,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                Some((bounds, 42))
            ),
            InputDecision::Show(bounds, 42)
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x5b,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::None
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x5c,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::Hide
        );
        assert_eq!(
            decide_event(
                true,
                InputEvent::Key {
                    code: 0x41,
                    down: true,
                    foreground: 1
                },
                &mut ignored,
                None
            ),
            InputDecision::Hide
        );
    }

    #[test]
    fn 마우스_down은_표시된_창을_숨긴다() {
        assert_eq!(
            decide_event(true, InputEvent::MouseDown, &mut [false; 256], None),
            InputDecision::Hide
        );
        assert_eq!(
            decide_event(false, InputEvent::MouseDown, &mut [false; 256], None),
            InputDecision::None
        );
    }

    #[test]
    fn 표시된_native_창_handle을_등록하고_해제한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        set_input_overlay_window(Some((7, 123)));
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), Some((7, 123)));
        clear_input_overlay_window(8);
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), Some((7, 123)));
        clear_input_overlay_window(7);
        assert_eq!(*INPUT_OVERLAY_WINDOW.lock().unwrap(), None);
        set_input_overlay_window(None);
    }

    #[test]
    #[cfg(windows)]
    fn 저수준_input_hook을_설치하고_정리한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let monitor = InputMonitor::new().unwrap();
        drop(monitor);
        assert!(INPUT_EVENTS.lock().unwrap().is_none());
    }

    #[test]
    fn input_queue_포화는_stale_event를_비우고_hide한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let (sender, events) = sync_channel(4);
        sender
            .send(InputEvent::Key {
                code: 0x5b,
                down: true,
                foreground: 1,
            })
            .unwrap();
        sender.send(InputEvent::MouseDown).unwrap();
        let mut monitor = InputMonitor {
            ignored: [true; 256],
            events,
            blocked_until: None,
            stopping: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        INPUT_OVERFLOW.store(true, Ordering::Release);
        let path = PathBuf::new();
        let env = Environment {
            root: path.clone(),
            user: path.clone(),
            documents: path.clone(),
            videos: path.clone(),
            exe: path,
            packaged: false,
            portable: false,
        };
        assert_eq!(monitor.poll(true, &env), InputDecision::Hide);
        assert!(monitor.ignored.iter().all(|value| !value));
        assert!(matches!(
            monitor.events.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        sender.send(InputEvent::MouseDown).unwrap();
        assert_eq!(monitor.poll(false, &env), InputDecision::None);
        assert!(matches!(
            monitor.events.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn input_channel_단절은_재시작_신호를_반환한다() {
        let _guard = INPUT_TEST_LOCK.lock().unwrap();
        let (sender, events) = sync_channel(1);
        drop(sender);
        let mut monitor = InputMonitor {
            ignored: [false; 256],
            events,
            blocked_until: None,
            stopping: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        let path = PathBuf::new();
        let env = Environment {
            root: path.clone(),
            user: path.clone(),
            documents: path.clone(),
            videos: path.clone(),
            exe: path,
            packaged: false,
            portable: false,
        };
        assert_eq!(monitor.poll(false, &env), InputDecision::Unavailable);
    }
}
