use crate::{
    Result, app_services,
    blackbox::{self, Blackbox, EditorSession},
    boost::Boost,
    channel_ping::{ChannelPing, InputDecision, InputMonitor},
    dxvk_manager, graphics,
    inputs::Inputs,
    network_manager,
    rpc::Rpc,
    service_windows::Windows,
    settings, storage,
    workers::{Environment, read, remove},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct Service {
    pub env: Environment,
    pub rpc: Rpc,
    pub windows: Arc<Windows>,
    pub boost: Boost,
    pub blackbox: Mutex<Blackbox>,
    pub inputs: Mutex<Inputs>,
    pub channel_ping: Arc<ChannelPing>,
    pub dxvk: Mutex<dxvk_manager::Manager>,
    pub dxvk_runtime: RwLock<Value>,
    startup: OnceLock<()>,
    preparation: OnceLock<()>,
    prompt: Mutex<()>,
    reports_notified: Mutex<std::collections::HashSet<String>>,
    media: RwLock<(PathBuf, Option<EditorSession>)>,
    saving: AtomicBool,
    network: Mutex<()>,
    pub fixture: bool,
    pub fixture_state: Mutex<Value>,
    pub startup_tray: bool,
    updater: Arc<crate::updater::Updater>,
}
fn bool_arg(v: &Value) -> Result<bool> {
    v.as_bool()
        .ok_or_else(|| "설정 값이 올바르지 않습니다".into())
}
pub fn result(value: Result<Value>) -> Value {
    match value {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => json!({"ok":false,"error":{"message":e}}),
    }
}
static STARTUP_LOG: OnceLock<PathBuf> = OnceLock::new();
static LOG_WRITE: Mutex<()> = Mutex::new(());
fn log(error: impl std::fmt::Display) {
    eprintln!("{error}");
    if let Some(path) = STARTUP_LOG.get() {
        if let Ok(_guard) = LOG_WRITE.lock() {
            let _ = append_log(path, &error.to_string());
        }
    }
}
fn append_log(path: &Path, message: &str) -> Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let rotate = std::fs::metadata(path).is_ok_and(|m| m.len() > 1024 * 1024);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(!rotate)
        .truncate(rotate)
        .open(path)
        .map_err(|e| e.to_string())?;
    let message: String = message.chars().take(16384).collect();
    writeln!(file, "{} {message}", app_services::iso()).map_err(|e| e.to_string())
}
struct SaveGuard<'a>(&'a AtomicBool);
impl Drop for SaveGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
fn sync_overlay_window(
    tracked: &mut Option<u64>,
    current: Option<u64>,
    visible: &mut bool,
) {
    if *tracked != current {
        *tracked = current;
        *visible = false;
    }
}
fn mark_overlay_shown(tracked: &mut Option<u64>, visible: &mut bool, window_id: u64) {
    *tracked = Some(window_id);
    *visible = true;
}
impl Service {
    pub fn new(
        env: Environment,
        rpc: Rpc,
        displays: Value,
        fixture: bool,
        startup_tray: bool,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak: &std::sync::Weak<Self>| {
            let w = weak.clone();
            let event = Arc::new(move |name: &str, value: Value| {
                let Some(service) = w.upgrade() else { return };
                let name = name.to_owned();
                std::thread::spawn(move || match name.as_str() {
                    "blackbox-shortcut" => {
                        if let Err(e) = service.save_clip("") {
                            log(e);
                        }
                    }
                    "blackbox-notification" => {
                        if let Err(e) = service.windows.notice(&value) {
                            log(e);
                        }
                    }
                    _ => {}
                });
            });
            let blackbox = Blackbox::new(env.clone(), event);
            let channel_ping = Arc::new(ChannelPing::new(&env));
            let clips = blackbox.clips();
            let updates = rpc.clone();
            let update_service=weak.clone();
            let updater = crate::updater::Updater::new(env.portable,Arc::new(move |value| {
                updates.event(1,"application:update-state-changed",json!([value]));
                if value["phase"]=="available" {if let Some(service)=update_service.upgrade() {
                    let version=value["version"].as_str().unwrap_or("").to_owned();
                    std::thread::spawn(move || {if let Err(e)=service.windows.update_notice(&version){log(e);}});
                }}
            }));
            Self {
                windows: Arc::new(Windows::new(env.clone(), rpc.clone(), displays)),
                boost: Boost::new(env.clone()),
                inputs: Mutex::new(Inputs::new(env.clone())),
                channel_ping,
                dxvk: Mutex::new(dxvk_manager::Manager::new(env.clone())),
                dxvk_runtime: RwLock::new(
                    json!({"state":"checking","latestVersion":null,"error":null}),
                ),
                blackbox: Mutex::new(blackbox),
                media: RwLock::new((clips, None)),
                env,
                rpc,
                startup: OnceLock::new(),
                preparation: OnceLock::new(),
                prompt: Mutex::new(()),
                reports_notified: Mutex::new(std::collections::HashSet::new()),
                saving: AtomicBool::new(false),
                network: Mutex::new(()),
                fixture,
                fixture_state: Mutex::new(crate::service_fixture::initial()),
                startup_tray,
                updater,
            }
        })
    }
    fn sync_media(&self, b: &Blackbox) {
        *self.media.write().unwrap() = (b.clips(), b.session.clone());
    }
    fn save_guard(&self) -> Result<SaveGuard<'_>> {
        if self.updater.state()["phase"]=="installing" {return Err("업데이트 설치를 준비하고 있습니다".into());}
        if self.saving.swap(true, Ordering::SeqCst) {
            Err("이전 클립을 저장하고 있습니다".into())
        } else {
            Ok(SaveGuard(&self.saving))
        }
    }
    fn save_clip(&self, name: &str) -> Result<Value> {
        let _guard = self.save_guard()?;
        self.blackbox.lock().unwrap().quick_clip(name)
    }
    pub fn protocol(&self, p: &Value) -> Result<Value> {
        let url = reqwest::Url::parse(p["url"].as_str().ok_or("영상 주소 누락")?)
            .map_err(|e| e.to_string())?;
        let path = {
            let state = self.media.read().unwrap();
            let host = url.host_str().unwrap_or("");
            if matches!(host, "clips" | "nogirem-blackbox.clips") {
                blackbox::clip_path(
                    &state.0,
                    &urlencoding::decode(url.path().trim_start_matches('/'))
                        .map_err(|e| e.to_string())?,
                )?
            } else {
                let parts: Vec<_> = url.path().split('/').filter(|s| !s.is_empty()).collect();
                if !matches!(host, "editor" | "nogirem-blackbox.editor") || parts.len() != 2 {
                    return Err("허용되지 않은 블랙박스 영상 요청입니다".into());
                }
                state
                    .1
                    .as_ref()
                    .filter(|s| s.id == parts[0])
                    .and_then(|s| s.files.get(parts[1]))
                    .cloned()
                    .ok_or("허용되지 않은 블랙박스 영상 요청입니다")?
            }
        };
        let range = p["headers"]
            .as_object()
            .and_then(|h| h.iter().find(|(k, _)| k.eq_ignore_ascii_case("range")))
            .and_then(|(_, v)| v.as_str());
        blackbox::video_response(&path, p["method"].as_str().unwrap_or("GET"), range)
    }
    fn initialize_boost(&self) {
        self.startup.get_or_init(|| {
            if let Err(e) = self.boost.set_enabled(true, false) {
                log(e);
            }
        });
    }
    fn prepare(self: &Arc<Self>) {
        self.preparation.get_or_init(|| {
            if self.env.packaged {
                if let Err(error)=app_services::startup(&self.env.exe,true,None){log(error);}
            }
            {
                let mut b = self.blackbox.lock().unwrap();
                let enabled = settings::blackbox(&read(
                    &self.env.user.join("blackbox/settings.json"),
                ))["enabled"]
                    == true;
                if enabled {
                    for attempt in 0..3 {
                        match b.start() {
                            Ok(_) => break,
                            Err(e) => {
                                log(e);
                                if attempt < 2 {
                                    std::thread::sleep(Duration::from_secs(attempt + 1));
                                }
                            }
                        }
                    }
                }
                self.sync_media(&b);
            }
            {
                let mut dxvk = self.dxvk.lock().unwrap();
                if let Err(e) = dxvk.load_local() {
                    dxvk.runtime =
                        json!({"state":"unavailable","latestVersion":null,"error":{"message":e}});
                }
            }
            self.notify_dxvk();
            match self.windows.open("dxvk-manager", false) {
                Ok(id) => {
                    let deadline = std::time::Instant::now() + Duration::from_secs(20);
                    while self.windows.get(id).is_some_and(|w| !w.ready)
                        && std::time::Instant::now() < deadline
                    {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
                Err(e) => log(e),
            }
            let service = self.clone();
            std::thread::spawn(move || {
                if let Err(e) = service.dxvk.lock().unwrap().refresh() {
                    log(e);
                }
                service.notify_dxvk();
            });
            if let Err(e) = crate::muo::install(
                &self.env.root,
                &self.env.documents,
                "주변캐릭터간소화프레임제한해제.muo",
            ) {
                log(e);
            }
        });
    }
    fn notify_dxvk(&self) {
        let runtime = self.dxvk.lock().unwrap().runtime.clone();
        *self.dxvk_runtime.write().unwrap() = runtime.clone();
        self.rpc
            .event(1, "optimization:dxvk-status-changed", json!([runtime]));
    }
    fn affinity(&self, refresh: bool) -> Result<Value> {
        let mut v = self.boost.affinity_status(refresh)?;
        v["dxvk"] = self.dxvk_runtime.read().unwrap().clone();
        Ok(v)
    }
    fn affinity_runtime(&self, ensure: bool) -> Result<Value> {
        let mut v = if ensure {
            self.boost.ensure_affinity()?
        } else {
            self.boost.affinity_runtime()?
        };
        v["dxvk"] = self.dxvk_runtime.read().unwrap().clone();
        Ok(v)
    }
    fn optimization(&self) -> Value {
        self.initialize_boost();
        std::thread::scope(|scope| {
            let g = scope.spawn(|| result(graphics::run(&self.env.root, &self.env.game(), false)));
            let n = scope.spawn(|| {
                result(network_manager::dispatch(
                    "status",
                    &self.env.user.join("network/fast-ping-original.json"),
                ))
            });
            let a = scope.spawn(|| result(self.affinity(true)));
            let m = scope.spawn(|| result(self.boost.memory_status()));
            let graphics = g.join().unwrap();
            json!({"graphics":graphics,"nvidia":graphics,"network":n.join().unwrap(),"affinity":a.join().unwrap(),"memory":m.join().unwrap()})
        })
    }
    fn launch_context(self: &Arc<Self>) -> Result<Value> {
        std::thread::scope(|scope| {
            let preparation = {
                let service = self.clone();
                scope.spawn(move || service.prepare())
            };
            let music = scope.spawn(|| {
                app_services::preference(
                    &self.env.user,
                    "get-startup-music-setting",
                    &Value::Null,
                )
            });
            let optimization = scope.spawn(|| self.optimization());
            preparation.join().map_err(|_| "시작 준비 작업이 중단되었습니다")?;
            let music = music
                .join()
                .map_err(|_| "시작 음악 설정 조회가 중단되었습니다")??;
            let optimization = optimization
                .join()
                .map_err(|_| "최적화 상태 조회가 중단되었습니다")?;
            Ok(json!({"startupTray":self.startup_tray,"startupMusicMuted":music["muted"],"optimizationStatus":optimization,"dxvk":self.dxvk_runtime.read().unwrap().clone(),"blackboxSetting":self.blackbox.lock().unwrap().status().ok()}))
        })
    }
    pub fn start(self: &Arc<Self>) -> Result<()> {
        self.windows.tray(true);
        self.windows.open_main(self.startup_tray)?;
        crate::update_install::mark_healthy(&std::env::args().collect::<Vec<_>>())?;
        if self.fixture {
            return Ok(());
        }
        let service = self.clone();
        std::thread::spawn(move || service.initialize_boost());
        let service = self.clone();
        std::thread::spawn(move || {
            if let Err(e) = service.inputs.lock().unwrap().start_saved() {
                log(e);
            }
        });
        let service = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            if !service.rpc.closed() {
                service.prepare();
            }
        });
        let service = self.clone();
        std::thread::spawn(move || service.monitor());
        let service = self.clone();
        std::thread::spawn(move || {
            service
                .channel_ping
                .run(|| service.rpc.closed() || service.boost.exiting.load(Ordering::SeqCst));
        });
        let service = self.clone();
        std::thread::spawn(move || service.monitor_channel_ping_overlay());
        let service = self.clone();
        std::thread::spawn(move || service.check_startup_network());
        if self.env.packaged {
            let service=self.clone();std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(crate::updater::STARTUP_CHECK_DELAY_MS));
                if !service.rpc.closed() && !service.boost.exiting.load(Ordering::SeqCst) {
                    let announcements=service.clone();std::thread::spawn(move || announcements.refresh_announcements());
                    let _=service.updater.request();
                }
            });
        }
        Ok(())
    }

    fn monitor_channel_ping_overlay(self: &Arc<Self>) {
        let mut input = None;
        let mut overlay_visible = false;
        let mut overlay_window_id = None;
        while !self.rpc.closed() && !self.boost.exiting.load(Ordering::SeqCst) {
            let enabled = self.channel_ping.enabled();
            if !enabled {
                input = None;
                if overlay_visible || self.windows.channel_ping_visible() {
                    self.windows.hide_channel_ping();
                }
                overlay_visible = false;
                overlay_window_id = None;
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            if input.is_none() {
                if let Err(error) = self.windows.prepare_channel_ping() {
                    self.channel_ping.set_input_error(Some(error.clone()));
                    log(format!("채널 핑 오버레이 준비 실패: {error}"));
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
                match InputMonitor::new() {
                    Ok(monitor) => {
                        self.channel_ping.set_input_error(None);
                        input = Some(monitor);
                    }
                    Err(error) => {
                        self.channel_ping.set_input_error(Some(error.clone()));
                        log(format!("채널 핑 입력 감시 시작 실패: {error}"));
                        std::thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                }
            }
            sync_overlay_window(
                &mut overlay_window_id,
                self.windows.channel_ping_id(),
                &mut overlay_visible,
            );
            match input
                .as_mut()
                .unwrap()
                .poll(overlay_visible, &self.env)
            {
                InputDecision::Show(bounds) => {
                    if self.channel_ping.enabled() {
                        match self.windows.show_channel_ping(bounds) {
                            Ok(window_id) => mark_overlay_shown(
                                &mut overlay_window_id,
                                &mut overlay_visible,
                                window_id,
                            ),
                            Err(error) => {
                                overlay_visible = false;
                                log(format!("채널 핑 오버레이 표시 실패: {error}"));
                            }
                        }
                    }
                    if !self.channel_ping.enabled() {
                        self.windows.destroy_channel_ping();
                        overlay_visible = false;
                        overlay_window_id = None;
                    }
                }
                InputDecision::Hide => {
                    self.windows.hide_channel_ping();
                    overlay_visible = false;
                }
                InputDecision::Unavailable => {
                    self.windows.hide_channel_ping();
                    overlay_visible = false;
                    self.channel_ping
                        .set_input_error(Some("입력 감시 작업이 중단되었습니다".into()));
                    input = None;
                    std::thread::sleep(Duration::from_secs(1));
                }
                InputDecision::None => {}
            }
        }
        self.windows.hide_channel_ping();
    }
    fn check_startup_network(&self) {
        let path = self.env.user.join("network/fast-ping-original.json");
        let saved = read(&path);
        if saved["interfaceGuid"].is_string() && saved["interfaceIndex"].as_i64().is_some() {
            if crate::network::connectivity(10, 3000).is_ok_and(|v| v["healthy"] != true) {
                let _lock = self.network.lock().unwrap();
                let message=match network_manager::dispatch("restore",&path){Ok(_)=>"앱 시작 검사에서 연결 이상이 반복되어 저장된 어댑터 설정을 원래 값으로 복원하고 네트워크 어댑터를 다시 시작했습니다.".into(),Err(e)=>format!("자동 복원에 실패했습니다. 네트워크 설정의 '설정 되돌리기'를 실행해 주세요.\n\n{e}")};
                self.network_warning(&message);
            }
        }
    }
    fn network_warning(&self, message: &str) {
        let _=self.rpc.request("dialog.showMessageBox",json!({"windowId":1,"options":{"type":"warning","title":"패스트핑 설정 자동 복원","message":"인터넷 연결 이상을 감지했습니다","detail":message,"buttons":["확인"],"defaultId":0,"noLink":true}}));
    }
    fn monitor(self: &Arc<Self>) {
        let directory = self.env.user.join("instance");
        let focus = directory.join("focus-request.json");
        let close = directory.join("installer-close-request");
        let mut request_at = read(&focus)["requestedAt"].as_u64().unwrap_or(0);
        let mut close_request = std::fs::read_to_string(&close).ok();
        let mut game = self.env.game();
        let mut dxvk_at = crate::now_ms();
        let mut announcements_at = crate::now_ms();
        let _ = storage::write_json(
            &directory.join("primary.json"),
            &json!({"pid":std::process::id(),"startedAt":crate::now_ms()}),
        );
        while !self.rpc.closed() && !self.boost.exiting.load(Ordering::SeqCst) {
            let request = read(&focus);
            let now = request["requestedAt"].as_u64().unwrap_or(0);
            if now > request_at {
                request_at = now;
                if request["shouldFocus"] != false {
                    self.windows.focus_main();
                }
                let _ = storage::write_json(
                    &directory.join("focus-acknowledgement.json"),
                    &json!({"requestId":request["requestId"],"acknowledgedAt":crate::now_ms(),"primaryPid":std::process::id()}),
                );
            }
            let next_close_request = std::fs::read_to_string(&close).ok();
            if next_close_request.is_some() && next_close_request != close_request {
                close_request = next_close_request;
                // 클립 저장으로 종료가 미뤄질 수 있으므로 실제 종료가 시작될 때까지 요청 감시를 유지한다.
                match self.finish_exit("keep") {
                    Ok(value) if value["closing"] == true => {
                        let _ = remove(&close);
                        break;
                    }
                    Err(error) => log(error),
                    _ => {}
                }
            }
            let next_game = self.env.game();
            if !next_game.eq_ignore_ascii_case(&game) {
                game = next_game;
                let service = self.clone();
                std::thread::spawn(move || {
                    if let Ok(v) = graphics::run(&service.env.root, &service.env.game(), false) {
                        service
                            .rpc
                            .event(1, "optimization:graphics-status-changed", json!([v]));
                    }
                    if let Err(e) = service.inputs.lock().unwrap().restart_input() {
                        log(e);
                    }
                    if let Err(e) = service.dxvk.lock().unwrap().refresh() {
                        log(e);
                    }
                    service.notify_dxvk();
                });
            }
            if crate::now_ms().saturating_sub(dxvk_at) >= 360000 {
                dxvk_at = crate::now_ms();
                let service = self.clone();
                std::thread::spawn(move || {
                    if let Err(e) = service.dxvk.lock().unwrap().refresh() {
                        log(e);
                    }
                    service.notify_dxvk();
                });
            }
            if crate::now_ms().saturating_sub(announcements_at) >= crate::updater::CHECK_INTERVAL_MS {
                announcements_at = crate::now_ms();
                let service = self.clone();
                std::thread::spawn(move || { if service.env.packaged { service.updater.check(); } service.refresh_announcements(); });
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    pub fn close_request(self: &Arc<Self>, native: bool) -> Result<Value> {
        if self.saving.load(Ordering::SeqCst) {
            return Ok(json!({"closing":false,"clipSaveInProgress":true}));
        }
        if self.boost.exiting.load(Ordering::SeqCst)
            || self.boost.close_pending.swap(true, Ordering::SeqCst) && !native
        {
            return Ok(Value::Null);
        }
        if !self.fixture && self.boost.exit_confirmation().unwrap_or(true) == false {
            return self.finish_exit("keep");
        }
        if native {
            let response=self.rpc.request("dialog.showMessageBox",json!({"options":{"type":"question","title":"프로그램 종료","message":"프레임 부스트를 정지할까요?","detail":"부스트를 유지한 채로 종료하면 마비노기 외 프로그램들의 성능이 제한될 수 있습니다.","buttons":["부스트 설정 되돌린 후 종료","적용 유지 후 종료","취소"],"defaultId":0,"cancelId":2,"noLink":true}}))?;
            return self.finish_exit(match response["response"].as_u64() {
                Some(0) => "reset",
                Some(1) => "keep",
                _ => "cancel",
            });
        }
        self.rpc.event(1, "application:close-requested", json!([]));
        Ok(json!(true))
    }
    pub fn finish_exit(&self, action: &str) -> Result<Value> {
        log(format!("EXIT requested action={action}"));
        if action == "cancel" {
            self.boost.close_pending.store(false, Ordering::SeqCst);
            return Ok(json!({"closing":false}));
        }
        if !matches!(action, "keep" | "reset") {
            return Err("종료 방식이 올바르지 않습니다".into());
        }
        if self.saving.load(Ordering::SeqCst) {
            return Ok(json!({"closing":false,"clipSaveInProgress":true}));
        }
        if self.boost.exiting.swap(true, Ordering::SeqCst) {
            return Ok(json!({"closing":true}));
        }
        self.updater.cancel();
        if !self.fixture {
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    if let Err(e) = self.boost.stop(action == "reset") {
                        log(e);
                    }
                });
                scope.spawn(|| {
                    if let Err(e) = self.inputs.lock().unwrap().stop() {
                        log(e);
                    }
                });
                scope.spawn(|| {
                    if let Err(e) = self.blackbox.lock().unwrap().stop() {
                        log(e);
                    }
                });
            });
        }
        log("EXIT workers stopped; requesting native host shutdown");
        self.rpc.native("application.exit", json!({"code":0}));
        Ok(json!({"closing":true}))
    }
    fn close_aux(self: &Arc<Self>, id: u64) {
        if self.fixture {
            self.windows.destroy(id);
            self.windows.focus_main();
        } else {
            self.windows.close(id);
        }
    }
    fn cleanup_session(self: &Arc<Self>) {
        let service = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(350));
            if service.windows.id("blackbox-manager").is_none()
                && service.windows.id("blackbox-editor").is_none()
            {
                let mut b = service.blackbox.lock().unwrap();
                b.session = None;
                service.sync_media(&b);
            }
        });
    }
    pub fn event(self: &Arc<Self>, name: &str, p: &Value) -> Result<()> {
        let id = p["windowId"].as_u64().unwrap_or(0);
        match name {
            "displays" => self
                .windows
                .displays(p["displays"].clone(), p["cursor"].clone()),
            "window" => {
                let event = p["event"].as_str().unwrap_or("");
                self.windows.event(id, event, &p["state"]);
                match event {
                    "close" => {
                        if id == 1 {
                            self.close_request(false)?;
                        } else if !self.saving.load(Ordering::SeqCst) {
                            self.close_aux(id);
                            self.cleanup_session();
                        }
                    }
                    "key" => {
                        if p["state"]["type"] == "keyDown"
                            && p["state"]["key"] == "Escape"
                            && p["state"]["isAutoRepeat"] != true
                            && id != 1
                        {
                            if self.windows.kind(id).as_deref() == Some("blackbox-manager") {
                                self.rpc
                                    .event(id, "blackbox-manager:escape-pressed", json!([]));
                            } else if !self.saving.load(Ordering::SeqCst) {
                                self.close_aux(id);
                                self.cleanup_session();
                            }
                        }
                    }
                    "minimize" if id == 1 => {
                        if !self.saving.load(Ordering::SeqCst) {
                            self.windows.minimize();
                            self.cleanup_session();
                        }
                    }
                    "focus" | "blur" | "show" | "hide" | "restore" => self.windows.notify_visual(),
                    _ => {}
                }
            }
            "send" => {
                let channel = p["channel"].as_str().unwrap_or("");
                if channel == "dxvk:content-ready"
                    && self.windows.kind(id).as_deref() == Some("dxvk-manager")
                {
                    self.windows.content_ready(id);
                } else if let Some((kind, action)) = channel.split_once(':') {
                    if matches!(kind, "character-guide" | "dxvk-guide")
                        && self.windows.kind(id).as_deref() == Some(kind)
                    {
                        self.windows.drag(id, action, &p["args"][0]);
                    }
                }
            }
            "tray" => {
                std::thread::sleep(Duration::from_millis(75));
                if p["event"] == "menu" && p["index"] == 1 {
                    let hidden = self.windows.is_hidden();
                    if !hidden {
                        self.windows.focus_main();
                    }
                    self.close_request(hidden)?;
                } else if p["event"] != "menu" || p["index"] == 0 {
                    self.windows.focus_main();
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn authorize(&self, id: u64, channel: &str) -> Result<()> {
        let kind = self.windows.kind(id).ok_or("Unknown native window")?;
        let prefix = channel.split(':').next().unwrap_or("");
        let allowed = match prefix {
            "application" | "optimization" | "smoke" => kind == "main",
            "dxvk" => kind == "dxvk-manager",
            "channel-ping" => kind == "channel-ping-overlay",
            prefix => prefix == kind,
        };
        if !allowed {
            return Err("허용되지 않은 창 요청입니다".into());
        }
        Ok(())
    }
    pub fn invoke(self: &Arc<Self>, p: &Value) -> Result<Value> {
        let id = p["windowId"].as_u64().ok_or("Missing window id")?;
        let channel = p["channel"].as_str().ok_or("Missing IPC channel")?;
        if std::env::var_os("NOGIREM_SERVICE_TRACE").is_some() {
            eprintln!("invoke: {channel}");
        }
        let args = &p["args"];
        let v = &args[0];
        self.authorize(id, channel)?;
        if self.fixture {
            if let Some(reply) = crate::service_fixture::invoke(self, id, channel, args) {
                return reply;
            }
        }
        let action = channel.split_once(':').map(|(_, a)| a).unwrap_or("");
        if channel.starts_with("application:open-") {
            let kind = action.strip_prefix("open-").unwrap();
            if matches!(
                kind,
                "character-guide"
                    | "dxvk-guide"
                    | "dxvk-manager"
                    | "blackbox-manager"
                    | "blackbox-editor"
            ) {
                self.windows.open(kind, true)?;
                return Ok(json!(true));
            }
        }
        if action == "request-close" && id != 1 {
            if self.saving.load(Ordering::SeqCst) {
                return Ok(json!(false));
            }
            self.close_aux(id);
            self.cleanup_session();
            return Ok(json!(true));
        }
        if channel.starts_with("blackbox-manager:") || channel.starts_with("blackbox-editor:") {
            return self.blackbox_invoke(id, action, args);
        }
        match channel{
   "application:begin-startup-reveal"=>{if !self.startup_tray{self.windows.reveal_main();}Ok(json!(true))},
   "application:complete-startup-animation"=>Ok(json!({"dxvk":self.dxvk_runtime.read().unwrap().clone()})),
   "application:get-launch-context"=>self.launch_context(),
   "optimization:get-status"=>Ok(self.optimization()),
   "optimization:refresh-graphics"|"optimization:refresh-nvidia"=>graphics::run(&self.env.root,&self.env.game(),false),
   "optimization:optimize-graphics"|"optimization:optimize-nvidia"=>graphics::run(&self.env.root,&self.env.game(),true),
   "optimization:refresh-network"|"optimization:optimize-network"|"optimization:restore-network"=>{let _operation=self.network.lock().unwrap();let action=match action{"optimize-network"=>"apply","restore-network"=>"restore",_=>"status"};let result=network_manager::dispatch(action,&self.env.user.join("network/fast-ping-original.json"))?;if result["rolledBack"]==true{self.network_warning("네트워크 최적화 후 인터넷 연결 이상을 감지해 패스트핑 설정을 원래 값으로 복원했습니다.");}Ok(result)},
   "optimization:refresh-affinity"|"optimization:refresh-game-cpu-core-setting"=>self.affinity(true),
   "optimization:get-affinity-runtime"=>self.affinity_runtime(true),
   "optimization:get-memory-runtime"=>self.boost.ensure_memory(),
   "optimization:refresh-memory"=>self.boost.memory_status(),
   "optimization:set-affinity-enabled"=>self.boost.set_affinity(bool_arg(&v["enabled"])?,v["includeNic"]==true,false),
   "optimization:reset-affinity"=>{self.boost.desired.store(false,Ordering::SeqCst);self.windows.tray_icon(false);self.boost.set_affinity(false,false,true)},
   "optimization:set-frame-boost-enabled"|"optimization:reset-frame-boost"=>{let enabled=if action=="reset-frame-boost"{false}else{bool_arg(&v["enabled"])?};self.windows.tray_icon(enabled);let result=self.boost.set_enabled(enabled,v["includeNic"]==true);self.windows.tray_icon(self.boost.desired.load(Ordering::SeqCst));result},
   "optimization:set-game-cpu-core-count"=>self.boost.set_core_count(v),"optimization:run-cpu-reorder"=>self.boost.reorder(),"optimization:set-memory-enabled"=>self.boost.set_memory(bool_arg(v)?,false),
   "optimization:get-dxvk-runtime-status"=>Ok(self.dxvk_runtime.read().unwrap().clone()),
   "application:get-startup-tray-setting"=>app_services::startup(&self.env.exe,self.env.packaged,None),"application:set-startup-tray-setting"=>app_services::startup(&self.env.exe,self.env.packaged,Some(bool_arg(v)?)),
   "application:get-turbo-key-setting"=>self.inputs.lock().unwrap().turbo_status(),"application:set-turbo-key-setting"=>self.inputs.lock().unwrap().set_turbo(v),"application:download-turbo-key-helper"=>self.inputs.lock().unwrap().install(env!("CARGO_PKG_VERSION")),"application:remove-turbo-key-helper"=>self.inputs.lock().unwrap().uninstall(),
   "application:get-input-guard-setting"=>self.inputs.lock().unwrap().input_status(),"application:set-input-guard-setting"=>self.inputs.lock().unwrap().set_input(v),
   "application:get-channel-ping-setting"|"channel-ping:get-status"=>Ok(self.channel_ping.status()),"application:set-channel-ping-setting"=>{let enabled=bool_arg(v)?;if enabled {self.windows.prepare_channel_ping()?;}let status=self.channel_ping.set_enabled(enabled)?;if !enabled {self.windows.destroy_channel_ping();}Ok(status)},
   "application:get-blackbox-setting"=>self.blackbox.lock().unwrap().status(),"application:set-blackbox-setting"=>{let mut b=self.blackbox.lock().unwrap();let result=b.set(v);self.sync_media(&b);result},
   "application:set-blackbox-enabled"=>self.blackbox.lock().unwrap().enabled(bool_arg(v)?),"application:set-blackbox-feature-enabled"=>self.blackbox.lock().unwrap().set(&json!({"featureEnabled":bool_arg(v)?,"enabled":bool_arg(v)?})),
   "application:save-blackbox-clip"=>self.save_clip(""),"application:clear-blackbox-recording"=>self.clear_recordings(id),"application:open-blackbox-folder"=>self.open_clips(),
   "application:get-visual-activity"=>Ok(json!(self.windows.visual_active())),
   "application:request-close"=>self.close_request(false),"application:confirm-close"=>self.finish_exit(v.as_str().unwrap_or("")),"application:minimize-to-tray"=>{if self.saving.load(Ordering::SeqCst){return Ok(json!(false));}self.windows.minimize();self.cleanup_session();Ok(json!(true))},
   "application:get-creator-channel"=>app_services::channel(&self.env.user),"application:get-notice"=>self.notice(),"application:get-report-responses"=>self.reports(),
   "application:record-creator-prompt-display"=>Ok(json!(false)),
   "application:dismiss-creator-prompt"=>{let _guard=self.prompt.lock().unwrap();app_services::preference(&self.env.user,action,v)},
   "application:set-startup-music-setting"|"application:get-creator-prompt-dismissed"|"application:dismiss-notice"|"application:acknowledge-report-response"=>app_services::preference(&self.env.user,action,v),
   "application:open-creator-channel"=>self.external(app_services::CHANNEL),"application:open-direct-donation"=>self.external("https://thedirectdonation.org/"),"application:open-operation-policy"=>self.external("https://mabinogi.nexon.com/page/archive/guide_view.asp?id=4889849&num=7&playtarget=1"),"application:open-bug-report-form"=>self.external("https://docs.google.com/forms/d/e/1FAIpQLSfx6-QVqsxgUDKsYCMAyg7A51ZYBMrMa_17OGzzQF_gGOum1w/viewform?usp=publish-editor"),
   "application:open-notice-link"=>{let url=v.as_str().unwrap_or("");let parsed=reqwest::Url::parse(url).map_err(|e|e.to_string())?;if !matches!(parsed.scheme(),"http"|"https"){return Ok(json!(false));}self.external(url)},
   "application:check-update"=>{let service=self.clone();std::thread::spawn(move || service.refresh_announcements());self.updater.request()},"application:get-update-state"=>Ok(self.updater.state()),"application:install-update"=>self.install_update(),
   "dxvk:get-status"=>self.dxvk.lock().unwrap().status(false),"dxvk:check-update"=>{let result={let mut dxvk=self.dxvk.lock().unwrap();let result=dxvk.status(true)?;dxvk.refresh()?;result};self.notify_dxvk();Ok(result)},"dxvk:install-update"=>{let result=self.dxvk.lock().unwrap().install(v.as_str().ok_or("버전 누락")?);self.notify_dxvk();result},
   "application:export-diagnostic-logs"=>crate::diagnostics::export(self),
   _=>Err(format!("Unknown IPC channel: {channel}"))
  }
    }
    fn install_update(&self) -> Result<Value> {
        if self.updater.state()["phase"]!="downloaded" {return Ok(json!(false));}
        let guard=self.save_guard()?;
        self.blackbox.lock().unwrap().flush_for_update()?;
        let (_,envelope,file)=self.updater.prepared()?;
        let mut pids=vec![std::process::id()];
        if let Ok(pid)=std::env::var("NOGIREM_DESKTOP_PID").unwrap_or_default().parse::<u32>() {pids.push(pid);}
        if let Some(pid)=std::env::args().find_map(|arg|arg.strip_prefix("--portable-launcher-pid=").and_then(|value|value.parse::<u32>().ok())){pids.push(pid);}
        let dir=crate::update_install::prepare(&self.env.exe,&self.env.user,&envelope,&file,pids,self.env.portable)?;
        self.updater.installing();
        drop(guard);
        if let Err(e)=std::fs::write(dir.join("commit"),b"install") {self.updater.failed(e.to_string());return Err(e.to_string());}
        self.finish_exit("keep")
    }
    fn notice(&self) -> Result<Value> {
        let value = app_services::notice(&self.env.root, &self.env.user).unwrap_or_else(|error| {
            log(format!("공지사항 확인 실패: {error}")); Value::Null
        });
        if !value.is_null() { self.rpc.event(1, "application:notice-available", json!([value])); }
        Ok(value)
    }
    fn refresh_announcements(&self) {
        let _ = self.notice();
        let _ = self.reports();
    }
    fn reports(&self) -> Result<Value> {
        let mut notified = self.reports_notified.lock().unwrap();
        let replies = app_services::reports(&self.env.root, &self.env.user, self.env.packaged)
            .unwrap_or_else(|error| {
                log(error);
                json!([])
            });
        let replies: Vec<Value> = replies
            .as_array()
            .into_iter()
            .flatten()
            .filter(|reply| notified.insert(reply["responseId"].as_str().unwrap_or("").to_owned()))
            .cloned()
            .collect();
        if !replies.is_empty() {
            self.rpc.event(1, "application:report-responses-available", json!([replies]));
        }
        Ok(json!(replies))
    }
    fn external(&self, url: &str) -> Result<Value> {
        self.rpc
            .request("shell.openExternal", json!({"path":url}))?;
        Ok(json!(true))
    }
    fn open_clips(&self) -> Result<Value> {
        let path = self.media.read().unwrap().0.clone();
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        self.rpc.request("shell.openPath", json!({"path":path}))?;
        Ok(json!(true))
    }
    fn clear_recordings(&self, id: u64) -> Result<Value> {
        let reply=self.rpc.request("dialog.showMessageBox",json!({"windowId":id,"options":{"type":"warning","title":"청크 정리","message":"모든 드라이브의 녹화 청크를 정리할까요?","detail":"저장된 클립은 삭제하지 않습니다.","buttons":["취소","정리"],"defaultId":0,"cancelId":0,"noLink":true}}))?;
        let mut b = self.blackbox.lock().unwrap();
        if reply["response"] != 1 {
            let mut value = b.status()?;
            value["canceled"] = json!(true);
            return Ok(value);
        }
        b.clear()
    }
    fn blackbox_invoke(self: &Arc<Self>, id: u64, action: &str, args: &Value) -> Result<Value> {
        let v = &args[0];
        match action {
            "get-status" => self.blackbox.lock().unwrap().status(),
            "set-setting" => {
                let mut b = self.blackbox.lock().unwrap();
                let result = b.set(v);
                self.sync_media(&b);
                result
            }
            "set-enabled" => self.blackbox.lock().unwrap().enabled(bool_arg(v)?),
            "save-clip" => self.save_clip(""),
            "clear-recording" => self.clear_recordings(id),
            "open-folder" => self.open_clips(),
            "open-editor" => {
                self.windows.open("blackbox-editor", true)?;
                Ok(json!(true))
            }
            "get-editor-session" | "get-session" => {
                let mut b = self.blackbox.lock().unwrap();
                let result = b.prepare();
                self.sync_media(&b);
                result
            }
            "set-track-seconds" => {
                let mut b = self.blackbox.lock().unwrap();
                let result = b.track(v);
                self.sync_media(&b);
                result
            }
            "extract" => {
                let _guard = self.save_guard()?;
                let rpc = self.rpc.clone();
                self.blackbox.lock().unwrap().extract(
                    v,
                    Arc::new(move |p| {
                        rpc.event(id, "blackbox-editor:extract-progress", json!([p]))
                    }),
                )
            }
            "suggest-clip-name" => Ok(json!(self.blackbox.lock().unwrap().suggest_name()?)),
            "list-clips" => self.blackbox.lock().unwrap().list_clips(),
            "rename-clip" => self
                .blackbox
                .lock()
                .unwrap()
                .rename_clip(v.as_str().unwrap_or(""), args[1].as_str().unwrap_or("")),
            "delete-clip" => self
                .blackbox
                .lock()
                .unwrap()
                .delete_clip(v.as_str().unwrap_or("")),
            "open-clip" => {
                let path =
                    blackbox::clip_path(&self.media.read().unwrap().0, v.as_str().unwrap_or(""))?;
                if !path.is_file() {
                    return Err("클립 파일을 찾을 수 없습니다".into());
                }
                self.rpc.request("shell.openPath", json!({"path":path}))?;
                Ok(json!(true))
            }
            "show-output" => {
                let path = PathBuf::from(v.as_str().ok_or("잘못된 출력 경로")?);
                let dir = self.media.read().unwrap().0.clone();
                if path.parent().and_then(|p| p.canonicalize().ok()) != dir.canonicalize().ok()
                    || !path.is_file()
                {
                    return Err("허용되지 않은 블랙박스 파일 위치 요청입니다".into());
                }
                self.rpc
                    .request("shell.showItemInFolder", json!({"path":path}))?;
                Ok(json!(true))
            }
            "choose-clip-storage" => {
                self.blackbox.lock().unwrap().approved_clips = None;
                let dir = self.media.read().unwrap().0.clone();
                let selection=self.rpc.request("dialog.showOpenDialog",json!({"windowId":id,"options":{"title":"클립 저장 위치 선택","defaultPath":dir,"properties":["openDirectory","createDirectory"]}}))?;
                if selection["canceled"] == true {
                    return Ok(json!({"canceled":true}));
                }
                let path = selection["filePaths"][0]
                    .as_str()
                    .ok_or("클립 저장 경로를 선택하지 않았습니다")?;
                self.blackbox.lock().unwrap().approved_clips = Some(PathBuf::from(path));
                std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
                Ok(json!({"canceled":false,"clipStoragePath":path}))
            }
            "set-page" => self.windows.page(id, v.as_str().unwrap_or("")),
            "fit-media" => self.windows.fit_media(id, v),
            "report-playback" => {
                append_log(
                    &self.env.user.join("blackbox/blackbox-events.log"),
                    &json!({"event":"editor-playback","details":v}).to_string(),
                )?;
                Ok(json!(true))
            }
            _ => Err(format!("Unknown blackbox operation: {action}")),
        }
    }
}
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let folders = std::env::var("NOGIREM_KNOWN_FOLDERS")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let mut env = Environment::new(root.into(), &folders)?;
    if args.iter().any(|a| a == "--smoke-test") {
        let isolated =
            std::env::temp_dir().join(format!("nogirem-rust-ui-fixture-{}", std::process::id()));
        env.user = isolated.join("user");
        env.documents = isolated.join("documents");
        env.videos = isolated.join("videos");
    }
    env.packaged = std::env::var("NOGIREM_PACKAGED").as_deref() == Ok("1");
    let displays = std::env::var("NOGIREM_DISPLAYS")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!([]));
    let (rpc, messages) = Rpc::stdio();
    let service = Service::new(
        env,
        rpc.clone(),
        displays,
        args.iter().any(|a| a == "--smoke-test"),
        args.iter().any(|a| a == "--startup-tray"),
    );
    let contract_only = args.iter().any(|a| a == "--contract-test");
    if !contract_only && !service.fixture {
        let _ = STARTUP_LOG.set(service.env.user.join("logs/startup.log"));
        log(format!(
            "START Dioxus Desktop {} Rust backend",
            env!("CARGO_PKG_VERSION")
        ));
        std::panic::set_hook(Box::new(|info| log(format!("PANIC {info}"))));
    }
    if !contract_only {
        let startup = service.clone();
        std::thread::spawn(move || {
            if let Err(e) = startup.start() {
                log(e);
                startup.rpc.native("application.exit", json!({"code":1}));
            }
        });
    }
    let (event_sender, event_receiver) = std::sync::mpsc::channel::<Value>();
    let event_service = service.clone();
    std::thread::spawn(move || {
        for message in event_receiver {
            if let Err(e) =
                event_service.event(message["name"].as_str().unwrap_or(""), &message["params"])
            {
                log(e);
            }
        }
    });
    for message in messages {
        if message["type"] == "disconnect" {
            break;
        }
        if message["type"] == "event" {
            let _ = event_sender.send(message);
            continue;
        }
        let service = service.clone();
        std::thread::spawn(move || {
            if message["type"] == "request" {
                let reply = match message["method"].as_str() {
                    Some("contract") => serde_json::from_str(include_str!("service-contract.json"))
                        .map_err(|e| e.to_string()),
                    Some("invoke") => service.invoke(&message["params"]),
                    Some("protocol") => service.protocol(&message["params"]),
                    _ => Err("Unknown service method".into()),
                };
                service.rpc.reply(&message["id"], reply);
            } else if message["type"] == "event" {
                if let Err(e) =
                    service.event(message["name"].as_str().unwrap_or(""), &message["params"])
                {
                    log(e);
                }
            }
        });
    }
    if !contract_only && !service.fixture && !service.boost.exiting.load(Ordering::SeqCst) {
        let _ = service.finish_exit("keep");
    }
    Ok(())
}

#[cfg(test)]
mod channel_ping_overlay_tests {
    use super::{mark_overlay_shown, sync_overlay_window};

    #[test]
    fn 보조_창이_재생성되면_표시_상태를_초기화한다() {
        let mut tracked = Some(2);
        let mut visible = true;
        sync_overlay_window(&mut tracked, Some(3), &mut visible);
        assert_eq!(tracked, Some(3));
        assert!(!visible);

        visible = true;
        sync_overlay_window(&mut tracked, Some(3), &mut visible);
        assert!(visible);
    }

    #[test]
    fn 표시_중_창이_바뀌면_실제_창_id를_추적한다() {
        let mut tracked = Some(2);
        let mut visible = false;
        sync_overlay_window(&mut tracked, Some(3), &mut visible);
        mark_overlay_shown(&mut tracked, &mut visible, 3);
        sync_overlay_window(&mut tracked, Some(3), &mut visible);
        assert_eq!(tracked, Some(3));
        assert!(visible);
    }
}
