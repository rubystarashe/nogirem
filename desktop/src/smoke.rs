use crate::{bridge::Client, native::Host, ui::State};
use dioxus::prelude::*;
use serde_json::{Value, json};
use std::time::Duration;

async fn inspect(window: &dioxus::desktop::DesktopContext, script: &str) -> Result<Value, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    window
        .webview
        .evaluate_script_with_callback(script, move |value| {
            if let Some(sender) = sender.lock().unwrap().take() {
                let _ = sender.send(value);
            }
        })
        .map_err(|e| format!("WebView inspection failed: {e}; script={script}"))?;
    let text = tokio::time::timeout(Duration::from_secs(10), receiver)
        .await
        .map_err(|_| "WebView inspection timed out")?
        .map_err(|_| "WebView inspection canceled")?;
    serde_json::from_str(&text).map_err(|e| format!("{e}: {text}"))
}

pub async fn run(client: Client, host: Host, state: Signal<State>) -> Result<Value, String> {
    for _ in 0..100 {
        if state.peek().ready && !state.peek().services.is_null() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if std::env::args().any(|a| a == "--smoke-visual-activity") { return visual_activity_check(host,state).await; }
    if std::env::args().any(|a| a == "--smoke-creator-prompt") { return creator_prompt_check(client,host,state).await; }
    if std::env::args().any(|a| a == "--smoke-updater") { return updater_check(client,host,state).await; }
    if std::env::args().any(|a| a == "--smoke-announcements") { return announcements_check(host,state).await; }
    if std::env::args().any(|a| a == "--smoke-terms") { return terms_scroll_check(host,state).await; }
    if std::env::args().any(|a| a == "--smoke-boost") { return boost_mask_check(client,host,state).await; }
    if std::env::args().any(|a| a == "--smoke-escape") {
        return escape_check(client, host, state).await;
    }
    if std::env::args().any(|a| a == "--smoke-startup") {
        return startup_check(client, host, state).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main = host.0.borrow().main.clone();
    let before = inspect(&main, "({text:document.body.innerText, width:innerWidth, height:innerHeight, stylesheetLoaded:!!document.querySelector('link[href=\"/web/styles.css\"]')?.sheet, images:[...document.images].every(i=>i.complete&&i.naturalWidth>0)})").await?;
    if before["stylesheetLoaded"] != true || before["images"] != true {
        return Err(format!("Assets did not load: {before}"));
    }
    if !before["text"]
        .as_str()
        .unwrap_or("")
        .contains("부스트 대기중")
    {
        return Err(format!("Initial UI missing: {before}"));
    }
    let layout = layout_check(&main, state).await?;
    // Actual DOM event -> Dioxus handler -> service -> signal -> DOM.
    // Waiting status must remain disabled, matching the original app.
    let waiting_disabled =
        inspect(&main, "document.querySelector('.boost-text-area').disabled").await?;
    if waiting_disabled != true {
        return Err("Waiting boost control must be disabled".into());
    }
    client
        .invoke(
            1,
            "optimization:set-frame-boost-enabled",
            json!([{"enabled":false,"includeNic":true}]),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(2300)).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let paused = inspect(&main, "document.body.innerText").await?;
    if !paused.as_str().unwrap_or("").contains("부스트 적용 중단됨") {
        return Err(format!("State did not render: {paused}"));
    }
    client
        .invoke(1, "application:request-close", json!([]))
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let close = inspect(
        &main,
        "document.querySelector('[role=dialog]')?.innerText ?? ''",
    )
    .await?;
    if !close.as_str().unwrap_or("").contains("적용 유지 후 종료") {
        return Err(format!("Exit prompt missing: {close}"));
    }
    inspect(
        &main,
        "document.querySelector('.modal-close').click(); true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    client
        .invoke(1, "application:open-character-guide", json!([]))
        .await?;
    tokio::time::sleep(Duration::from_millis(700)).await;
    let auxiliary = host
        .0
        .borrow()
        .windows
        .iter()
        .find(|(id, _)| **id != 1)
        .map(|(_, w)| w.clone())
        .ok_or("Auxiliary window not created")?;
    let guide = inspect(&auxiliary, "({bridge:typeof window.characterGuide?.startDrag, text:document.body.innerText, images:[...document.images].map(i=>({src:i.src,loaded:i.complete&&i.naturalWidth>0}))})").await?;
    if guide["bridge"] != "function" {
        return Err(format!("Auxiliary bridge missing: {guide}"));
    }
    if guide["images"]
        .as_array()
        .is_none_or(|images| images.iter().any(|image| image["loaded"] != true))
    {
        return Err(format!("Auxiliary assets missing: {guide}"));
    }
    inspect(&auxiliary, "setTimeout(()=>window.close(),50); true").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if host.0.borrow().windows.len() != 1 {
        return Err("Auxiliary close did not release its native window".into());
    }
    let mut auxiliary_results = vec![];
    for (channel, object) in [
        ("dxvk-guide", "dxvkGuide"),
        ("dxvk-manager", "dxvkManager"),
        ("blackbox-manager", "blackboxManager"),
        ("blackbox-editor", "blackboxEditor"),
    ] {
        client
            .invoke(1, &format!("application:open-{channel}"), json!([]))
            .await?;
        tokio::time::sleep(Duration::from_millis(900)).await;
        let child = host
            .0
            .borrow()
            .windows
            .iter()
            .find(|(id, _)| **id != 1)
            .map(|(_, w)| w.clone())
            .ok_or("Auxiliary window not created")?;
        check_opacity_survives_show(&child).await?;
        let result = inspect(&child, &format!("({{bridge:typeof window.{object}, errors:window.__nogiremErrors, text:document.body.innerText, frameReady:document.querySelector('iframe')?.contentDocument?.querySelector('.editor')?.className}})")).await?;
        if result["bridge"] != "object"
            || result["errors"]
                .as_array()
                .is_none_or(|items| !items.is_empty())
        {
            return Err(format!("{channel}: {result}"));
        }
        if channel == "blackbox-manager"
            && !result["frameReady"]
                .as_str()
                .unwrap_or("")
                .contains("ready")
        {
            return Err(format!("Embedded editor did not initialize: {result}"));
        }
        if channel == "blackbox-editor"
            && !result["text"]
                .as_str()
                .unwrap_or("")
                .contains("녹화된 영상이 없습니다")
        {
            return Err(format!("Editor did not initialize: {result}"));
        }
        auxiliary_results.push(json!({"window":channel,"result":result}));
        inspect(&child, "setTimeout(()=>window.close(),50); true").await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    client
        .invoke(1, "application:minimize-to-tray", json!([]))
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    if main.is_visible() {
        return Err("Minimize did not hide the window".into());
    }
    activate_tray_menu(&host, 0)?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    if !main.is_visible() {
        return Err("Tray activation did not restore the window".into());
    }
    if host.0.borrow().tray.is_none() {
        return Err("Native tray icon missing".into());
    }
    activate_tray_menu(&host, 1)?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let tray_exit = inspect(
        &main,
        "document.querySelector('[role=dialog]')?.innerText ?? ''",
    )
    .await?;
    if !tray_exit
        .as_str()
        .unwrap_or("")
        .contains("적용 유지 후 종료")
    {
        return Err(format!(
            "Native tray exit menu did not request confirmation: {tray_exit}"
        ));
    }
    inspect(
        &main,
        "document.querySelector('.modal-close').click(); true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    inspect(
        &main,
        "document.querySelector('.creator-credit').click(); true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(900)).await;
    inspect(&main, "[...document.querySelectorAll('.creator-tabs button')].find(e=>e.textContent==='고급 기능').click(); true").await?;
    tokio::time::sleep(Duration::from_millis(900)).await;

    tokio::time::sleep(Duration::from_millis(900)).await;
    let advanced = inspect(&main, r#"(()=>{const panel=document.querySelector('.creator-content');return {headings:[...panel.querySelectorAll('h2')].map(e=>e.textContent),scrollable:panel.scrollHeight>panel.clientHeight,icons:document.querySelectorAll('.window-controls svg').length}})()"#).await?;
    if advanced["scrollable"] != true || advanced["icons"] != 4 {
        return Err(format!("Advanced structure failed: {advanced}"));
    }
    inspect(&main, "(()=>{const e=document.querySelector('[aria-label=\"게임 마우스 커서 크기\"]');e.scrollIntoView({block:'center'});e.value='200';e.dispatchEvent(new Event('change',{bubbles:true}));})();true").await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    if state.peek().input["cursorScalePercent"] != 200 {
        return Err("Inline cursor setting did not reach service".into());
    }
    inspect(&main, "[...document.querySelectorAll('.game-cpu-core-option')].find(e=>e.textContent==='3').click();true").await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    if state.peek().services["affinity"]["gameCoreSetting"]["gameCoreCount"] != 3 {
        return Err("Inline CPU setting did not reach service".into());
    }
    inspect(&main, "[...document.querySelectorAll('.developer-tool-row')].find(e=>e.querySelector('h2')?.textContent==='터보 키').querySelector('button').click();true").await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    if let Some(path) =
        std::env::args().find_map(|a| a.strip_prefix("--capture=").map(str::to_owned))
    {
        capture(&main, &format!("{path}-keyboard.png"))?;
    }
    inspect(&main, "(()=>{const e=document.querySelector('[role=dialog] select');e.value='5';e.dispatchEvent(new Event('change',{bubbles:true}));})();true").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    inspect(&main, "[...document.querySelectorAll('[role=dialog] button')].find(e=>e.textContent==='설정 완료').click();true").await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    if state.peek().turbo["intervalMs"] != 5 || state.peek().turbo["enabled"] != true {
        return Err("Turbo save did not reach service".into());
    }

    tokio::time::sleep(Duration::from_millis(300)).await;
    inspect(
        &main,
        "document.querySelector('.creator-content').scrollTop=0;true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(600)).await;
    if let Some(path) =
        std::env::args().find_map(|a| a.strip_prefix("--capture=").map(str::to_owned))
    {
        capture(&main, &path)?;
    }
    let error = state.peek().error.clone();
    if !error.is_empty() {
        return Err(error);
    }
    Ok(
        json!({"passed":true,"layout":layout,"initial":before,"paused":paused,"exitDialog":close,"auxiliary":guide,"otherWindows":auxiliary_results,"trayHideRestore":true,"nativeTrayMenuOpenAndExit":true,"settingsRoundtrip":true}),
    )
}

async fn startup_check(client: Client, host: Host, mut state: Signal<State>) -> Result<Value, String> {
    let main = host.0.borrow().main.clone();
    state.write().visual_active = false;
    let opening = inspect(&main, "({logo:!!document.querySelector('.game-wave.startup .game-wave-logo'),copy:document.querySelector('.game-wave-copy strong')?.textContent,guideEntered:document.querySelector('.character-guide-link')?.classList.contains('entered')})").await?;
    if opening["logo"] != true
        || opening["copy"] != "마비노기 렘 부스터"
        || opening["guideEntered"] == true
    {
        return Err(format!("Startup overlay/entry mismatch: {opening}"));
    }
    let border = inspect(
        &main,
        "getComputedStyle(document.querySelector('.game-wave'),'::after').boxShadow",
    )
    .await?;
    if !border.as_str().is_some_and(|v| v.contains("200, 200, 200")) {
        return Err(format!("Startup border missing: {border}"));
    }
    inspect(&main, r#"(()=>{const trace=window.__logoTrace={maskSeen:false,handoff:false,mismatches:0,opacities:[],stop:false};function sample(){const wave=window.nogiremWave?.inspect(),logo=document.querySelector('.boost-home>img');if(wave&&logo&&document.querySelector('.interface-ready')){const style=getComputedStyle(logo);if(wave.logoMask){trace.maskSeen=true;if(style.visibility!=='hidden')trace.mismatches++}else if(trace.maskSeen){trace.handoff=true;if(style.visibility==='hidden')trace.mismatches++;trace.opacities.push(Number(style.opacity))}}if(!trace.stop)requestAnimationFrame(sample)}sample();return true})()"#).await?;
    tokio::time::sleep(Duration::from_millis(1600)).await;
    let audio = inspect(&main, "window.nogiremWave.inspect()").await?;
    if audio["audioPaused"] != false
        || audio["audioTime"].as_f64().unwrap_or(0.0) < 18.5
        || audio["audioVolume"].as_f64().unwrap_or(0.0) <= 0.0
    {
        return Err(format!("Startup audio did not play: {audio}"));
    }
    if !main.is_visible() {
        return Err("Startup playback did not reveal main window".into());
    }
    let pos = main.outer_position().map_err(|e| e.to_string())?;
    let monitor = main.primary_monitor().ok_or("No monitor")?;
    let size = main.outer_size();
    let msize = monitor.size();
    let origin = monitor.position();
    if (pos.x - (origin.x + (msize.width as i32 - size.width as i32) / 2)).abs() > 3
        || (pos.y - (origin.y + (msize.height as i32 - size.height as i32) / 2)).abs() > 3
    {
        return Err(format!("Main window was not centered: {pos:?}"));
    }
    client.invoke(1, "smoke:prewarm-hidden", json!([])).await?;
    tokio::time::sleep(Duration::from_millis(400)).await;
    if host
        .0
        .borrow()
        .windows
        .iter()
        .any(|(id, w)| *id != 1 && w.is_visible())
    {
        return Err("Prewarmed auxiliary window became visible".into());
    }
    state.write().visual_active = false;
    let started = std::time::Instant::now();
    let mut done_at = None;
    let mut controls_at = None;
    let mut navigation_at = None;
    let mut guide_fade = false;
    let mut public_label_seen = false;
    while started.elapsed() < Duration::from_secs(10) {
        let now = started.elapsed().as_millis() as u64;
        if state.peek().identity == "version" && !public_label_seen {
            let text = inspect(
                &main,
                "document.querySelector('.boost-text-area').textContent",
            )
            .await?;
            if text.as_str() != Some("공개 사용자 버전") {
                return Err(format!("Wrong startup release label: {text}"));
            }
            public_label_seen = true;
        }
        if state.peek().identity == "done" && done_at.is_none() {
            done_at = Some(now);
        }
        if state.peek().controls_entered && controls_at.is_none() {
            controls_at = Some(now);
        }
        if let Some(at) = controls_at {
            if now > at + 200 && now < at + 400 {
                let opacity = inspect(&main, "Number(getComputedStyle(document.querySelector('.character-guide-link')).opacity)").await?;
                guide_fade |= opacity.as_f64().is_some_and(|v| v > 0.0 && v < 1.0);
            }
        }
        if state.peek().navigation_ready {
            navigation_at = Some(now);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    if !public_label_seen {
        return Err("Public release label was skipped".into());
    }
    let done = done_at.ok_or("Startup status missing")?;
    let controls_delay = controls_at
        .ok_or("Startup controls missing")?
        .saturating_sub(done);
    let navigation_delay = navigation_at
        .ok_or("Startup navigation missing")?
        .saturating_sub(done);
    if !(1950..=2300).contains(&controls_delay)
        || !(2900..=3300).contains(&navigation_delay)
        || !guide_fade
    {
        return Err(format!(
            "Startup timing mismatch: controls={controls_delay}, navigation={navigation_delay}, guide_fade={guide_fade}"
        ));
    }
    for _ in 0..40 {
        if inspect(&main,"window.nogiremWave.inspect().startupSequenceActive").await?==false {break;}
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let unfocused_startup=inspect(&main,"window.nogiremWave.inspect()").await?;
    if unfocused_startup["pageVisible"]!=false || unfocused_startup["startupSequenceActive"]!=false || unfocused_startup["logoMask"]!=false || unfocused_startup["renderedFrames"].as_u64()<=audio["renderedFrames"].as_u64(){return Err(format!("Unfocused startup did not finish: {unfocused_startup}"));}
    main.set_focus();
    tokio::time::sleep(Duration::from_millis(300)).await;
    state.write().visual_active=true;
    tokio::time::sleep(Duration::from_millis(200)).await;
    inspect(&main,"(()=>{const c=document.querySelector('#nogirem-wave').getBoundingClientRect();document.querySelector('.boost-home').dispatchEvent(new MouseEvent('mousemove',{bubbles:true,clientX:c.right,clientY:c.bottom}));setTimeout(()=>{window.__parallaxProbe=window.nogiremWave.inspect()},1500);return true})()").await?;
    tokio::time::sleep(Duration::from_millis(1800)).await;
    let parallax = inspect(&main, "window.__parallaxProbe").await?;
    if parallax["offsetX"].as_f64().unwrap_or(0.0) < 15.5
        || parallax["offsetY"].as_f64().unwrap_or(0.0) < 14.0
    {
        return Err(format!("Parallax displacement too small: {parallax}"));
    }
    inspect(&main,"window.__beforeWave=document.querySelector('canvas').toDataURL();window.dispatchEvent(new MouseEvent('mousedown',{clientX:100,clientY:180,bubbles:true}));true").await?;
    tokio::time::sleep(Duration::from_millis(180)).await;
    let ripple=inspect(&main,"({changed:window.__beforeWave!==document.querySelector('canvas').toDataURL(),state:window.nogiremWave.inspect()})").await?;
    if ripple["changed"] != true {
        return Err(format!("Click ripple did not change canvas: {ripple}"));
    }
    let logo_trace = inspect(&main, "window.__logoTrace.stop=true;window.__logoTrace").await?;
    let opacities = logo_trace["opacities"]
        .as_array()
        .ok_or("Missing logo trace")?;
    if logo_trace["maskSeen"] != true
        || logo_trace["handoff"] != true
        || logo_trace["mismatches"] != 0
        || !opacities
            .iter()
            .any(|v| v.as_f64().is_some_and(|a| a > 0.4 && a < 0.69))
    {
        return Err(format!("Logo handoff was not continuous: {logo_trace}"));
    }
    Ok(
        json!({"passed":true,"audio":audio,"ripple":ripple,"centered":true,"hiddenAuxiliary":true,"startupComplete":true,"unfocusedStartupComplete":true,"startupOverlay":opening,"controlsDelayMs":controls_delay,"navigationDelayMs":navigation_delay,"guideFade":guide_fade}),
    )
}

pub fn capture(window: &dioxus::desktop::DesktopContext, path: &str) -> Result<(), String> {
    use dioxus::desktop::tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::Graphics::Gdi::*;
    unsafe extern "system" {
        fn PrintWindow(hwnd: *mut std::ffi::c_void, dc: *mut std::ffi::c_void, flags: u32) -> i32;
    }
    let size = window.inner_size();
    unsafe {
        let dc = GetDC(window.hwnd() as _);
        let mem = CreateCompatibleDC(dc);
        let bitmap = CreateCompatibleBitmap(dc, size.width as i32, size.height as i32);
        let old = SelectObject(mem, bitmap);
        let ok = PrintWindow(window.hwnd() as _, mem, 3);
        SelectObject(mem, old);
        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = size.width as i32;
        info.bmiHeader.biHeight = -(size.height as i32);
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        let mut bytes = vec![0u8; (size.width * size.height * 4) as usize];
        let rows = GetDIBits(
            mem,
            bitmap,
            0,
            size.height,
            bytes.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );
        DeleteObject(bitmap);
        DeleteDC(mem);
        ReleaseDC(window.hwnd() as _, dc);
        if ok == 0 || rows == 0 {
            return Err("Window capture failed".into());
        }
        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        image::save_buffer_with_format(
            path,
            &bytes,
            size.width,
            size.height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())
    }
}

pub async fn audit(
    client: Client,
    host: Host,
    mut state: Signal<State>,
    directory: &str,
) -> Result<Value, String> {
    for _ in 0..600 {
        if state.peek().navigation_ready
            && state.peek().turbo.is_object()
            && state.peek().input.is_object()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if !state.peek().navigation_ready {
        return Err(format!(
            "Real startup did not finish: {}",
            state.peek().error
        ));
    }
    let main = host.0.borrow().main.clone();
    let root = std::path::Path::new(directory);
    tokio::time::sleep(Duration::from_secs(3)).await;
    let before=inspect(&main,r#"({text:document.body.innerText,summary:getComputedStyle(document.querySelector('.optimization-summary')).display,svg:document.querySelectorAll('.window-controls svg').length,wave:window.nogiremWave.inspect()})"#).await?;
    capture(&main, root.join("real-home.png").to_str().unwrap())?;
    if host
        .0
        .borrow()
        .windows
        .iter()
        .any(|(id, w)| *id != 1 && w.is_visible())
    {
        return Err("Real prewarmed window was visible".into());
    }
    inspect(
        &main,
        "document.querySelector('.creator-credit').click();true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(900)).await;
    inspect(&main,"[...document.querySelectorAll('.creator-tabs button')].find(e=>e.textContent==='고급 기능').click();true").await?;
    tokio::time::sleep(Duration::from_millis(600)).await;
    capture(&main, root.join("real-advanced.png").to_str().unwrap())?;
    let controls=inspect(&main,r#"({text:document.querySelector('.creator-content').innerText,scrollHeight:document.querySelector('.creator-content').scrollHeight,clientHeight:document.querySelector('.creator-content').clientHeight,disabled:[...document.querySelectorAll('.developer-tool-row')].map(e=>({label:e.querySelector('h2')?.textContent,disabled:e.querySelector('button')?.disabled}))})"#).await?;
    // A reversible real handler roundtrip; always restore the user's music setting.
    let original = state.peek().muted;
    let changed = client
        .invoke(
            1,
            "application:set-startup-music-setting",
            json!([!original]),
        )
        .await;
    let restored = client
        .invoke(
            1,
            "application:set-startup-music-setting",
            json!([original]),
        )
        .await?;
    if changed?["muted"] != !original || restored["muted"] != original {
        return Err("Real setting roundtrip failed".into());
    }
    let settings = json!({"turbo":state.peek().turbo,"input":state.peek().input,"blackbox":state.peek().blackbox,"affinity":state.peek().services["affinity"]});
    let error = state.peek().error.clone();
    state.write().tab.clear();
    state.write().creator_phase = "home".into();
    if !error.is_empty() {
        return Err(error);
    }
    if std::env::args().any(|a| a == "--window-audit") {
        window_audit(&client, &host, root).await?;
    }
    Ok(
        json!({"passed":true,"home":before,"advanced":controls,"settings":settings,"realSettingRoundtrip":true,"prewarmHidden":true}),
    )
}

async fn layout_check(
    main: &dioxus::desktop::DesktopContext,
    mut state: Signal<State>,
) -> Result<Value, String> {
    let prefix = std::env::args().find_map(|a| a.strip_prefix("--capture=").map(str::to_owned));
    let save = |name: &str| -> Result<(), String> {
        if let Some(prefix) = &prefix {
            capture(main, &format!("{prefix}-{name}.png"))?;
        }
        Ok(())
    };
    save("home")?;
    let blackbox=inspect(main,r#"({duration:document.querySelector('.blackbox-main-duration')?.textContent,textColor:getComputedStyle(document.querySelector('.blackbox-text-final')).backgroundColor,iconColor:getComputedStyle(document.querySelector('.blackbox-window-link svg')).backgroundColor})"#).await?;
    if blackbox["duration"] != "2분"
        || blackbox["textColor"] != "rgb(255, 212, 0)"
        || blackbox["iconColor"] != "rgb(255, 212, 0)"
    {
        return Err(format!(
            "Blackbox original structure/style missing: {blackbox}"
        ));
    }
    inspect(
        main,
        "document.querySelector('.optimization-summary').click();true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    let optimization=inspect(main,r#"(()=>{const sheet=document.querySelector('.modal-sheet'),r=sheet.getBoundingClientRect(),close=sheet.querySelector('.modal-close'),c=close.getBoundingClientRect();return {large:sheet.classList.contains('large'),headers:sheet.querySelectorAll('.optimization-detail>header .detail-action').length,extraButtons:sheet.querySelectorAll('.setting-action').length,overflowX:sheet.scrollWidth-sheet.clientWidth,overflowY:sheet.scrollHeight-sheet.clientHeight,top:r.top,bottom:r.bottom,closeVisible:c.top>=r.top&&c.bottom<=r.bottom,closeHit:close.contains(document.elementFromPoint(c.x+c.width/2,c.y+c.height/2))}})()"#).await?;
    save("optimization")?;
    if optimization["large"] != true
        || optimization["headers"] != 2
        || optimization["extraButtons"] != 0
        || optimization["overflowX"].as_f64().unwrap_or(99.) > 1.
        || optimization["overflowY"].as_f64().unwrap_or(99.) > 1.
        || optimization["closeVisible"] != true
        || optimization["closeHit"] != true
    {
        return Err(format!(
            "Original optimization sheet layout mismatch: {optimization}"
        ));
    }
    inspect(main, "document.querySelector('.modal-close').click();true").await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    inspect(
        main,
        "document.querySelector('.creator-credit').click();true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(850)).await;
    let mut tabs = Vec::new();
    for (id, label) in [
        ("developer", "소개"),
        ("operation", "작동 원리"),
        ("donation", "후원 / 기부"),
        ("history", "버전 변경 기록"),
        ("developer-tools", "고급 기능"),
    ] {
        inspect(main,&format!("[...document.querySelectorAll('.creator-tabs button')].find(e=>e.textContent==={}).click();true",json!(label))).await?;
        tokio::time::sleep(Duration::from_millis(220)).await;
        let during = inspect(
            main,
            "getComputedStyle(document.querySelector('.creator-content')).overflowX",
        )
        .await?;
        if during != "hidden" {
            return Err(format!(
                "Horizontal scrollbar exposed during {id} transition"
            ));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        let layout=inspect(main,r#"(()=>{const p=document.querySelector('.creator-content');return{overflowX:getComputedStyle(p).overflowX,scrollWidth:p.scrollWidth,width:p.clientWidth,height:p.clientHeight,scrollHeight:p.scrollHeight,visible:[...p.querySelectorAll('h1,h2,h3')].map(e=>e.textContent)}})()"#).await?;
        if layout["overflowX"] != "hidden" {
            return Err(format!("Horizontal overflow in {id}: {layout}"));
        }
        save(id)?;
        tabs.push(json!({"tab":id,"layout":layout}));
    }
    inspect(
        main,
        "document.querySelector('.creator-content').scrollTop=99999;true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    save("advanced-bottom")?;
    inspect(
        main,
        "document.querySelector('.creator-return').click();true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(850)).await;
    let previous_turbo = state.peek().turbo.clone();
    {
        let mut s = state.write();
        s.turbo["installed"] = json!(false);
        s.modal = "turbo".into();
    }
    tokio::time::sleep(Duration::from_millis(250)).await;
    save("terms")?;
    let terms=inspect(main,"({header:!!document.querySelector('.terms-modal>header'),footer:!!document.querySelector('.terms-modal>footer'),scroll:!!document.querySelector('.terms-scroll')})").await?;
    if terms["header"] != true || terms["footer"] != true || terms["scroll"] != true {
        return Err(format!("Terms modal structure mismatch: {terms}"));
    }
    {
        let mut s = state.write();
        s.turbo = previous_turbo;
        s.modal = "notice".into();
        s.notice = json!({"id":"layout-fixture","markdown":"# 공지사항\n\n공지 본문입니다.\n\n- 첫 번째 항목\n- 두 번째 항목"});
    }
    tokio::time::sleep(Duration::from_millis(250)).await;
    save("notice")?;
    {
        let mut s = state.write();
        s.modal.clear();
        s.notice = Value::Null;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(json!({"blackbox":blackbox,"optimization":optimization,"tabs":tabs,"terms":terms}))
}

async fn window_audit(client: &Client, host: &Host, root: &std::path::Path) -> Result<(), String> {
    let existing: Vec<u64> = host.0.borrow().windows.keys().copied().collect();
    client
        .invoke(1, "application:open-character-guide", json!([]))
        .await?;
    for _ in 0..100 {
        if host
            .0
            .borrow()
            .windows
            .keys()
            .any(|id| !existing.contains(id))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (_guide_id, guide) = host
        .0
        .borrow()
        .windows
        .iter()
        .find(|(id, _)| !existing.contains(id))
        .map(|(id, w)| (*id, w.clone()))
        .ok_or("Guide missing")?;
    let position = guide.outer_position().map_err(|e| e.to_string())?;
    let size = guide.outer_size();
    let monitor = guide.current_monitor().ok_or("Guide monitor missing")?;
    use dioxus::desktop::tao::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};
    let mut info = windows_sys::Win32::Graphics::Gdi::MONITORINFO {
        cbSize: std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as _,
        ..Default::default()
    };
    unsafe {
        windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(monitor.hmonitor() as _, &mut info);
    }
    let dx = position.x + (size.width as i32) / 2 - (info.rcWork.left + info.rcWork.right) / 2;
    let dy = position.y + (size.height as i32) / 2 - (info.rcWork.top + info.rcWork.bottom) / 2;
    inspect(
        &guide,
        "window.characterGuide.moveDrag(-9999,-9999);window.characterGuide.endDrag();true",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    if guide.outer_position().map_err(|e| e.to_string())? != position {
        return Err("Legacy drag coordinates moved native window".into());
    }
    capture(&guide, root.join("guide-window.png").to_str().unwrap())?;
    if dx.abs() > 2 || dy.abs() > 2 {
        return Err(format!("Guide not centered: {dx},{dy}"));
    }
    inspect(&guide, "setTimeout(()=>window.close(),50);true").await?;
    let existing: Vec<u64> = host.0.borrow().windows.keys().copied().collect();
    client
        .invoke(1, "application:open-blackbox-manager", json!([]))
        .await?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let (id, window) = host
        .0
        .borrow()
        .windows
        .iter()
        .find(|(id, _)| !existing.contains(id))
        .map(|(id, w)| (*id, w.clone()))
        .ok_or("Manager missing")?;
    // Opening the editor preserves the original recent 15-minute default.
    let session = client
        .invoke(id, "blackbox-manager:get-editor-session", json!([]))
        .await?;
    for _ in 0..600 {
        let ready=inspect(&window,"document.querySelector('iframe')?.contentDocument?.querySelector('.editor')?.className").await?;
        if ready.as_str().is_some_and(|s| s.contains("ready")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let status = client
        .invoke(id, "blackbox-manager:get-status", json!([]))
        .await?;
    std::fs::write(
        root.join("media-session.json"),
        serde_json::to_string_pretty(&session).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    // Initial opening itself must show recorded media without a recovery-button click.
    if session["timelineDurationSeconds"] != 900 {
        return Err("Default recording window is not 15 minutes".into());
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let media=inspect(&window,r#"(()=>{const d=document.querySelector('iframe').contentDocument,v=d.querySelector('video');return{width:v.videoWidth,height:v.videoHeight,readyState:v.readyState,currentTime:v.currentTime,error:v.error?.message,gapHidden:d.querySelector('.gap-preview').hidden,editor:d.querySelector('.editor').className}})()"#).await?;
    if session["segments"]
        .as_array()
        .is_some_and(|s| !s.is_empty())
        && (media["gapHidden"] != true
            || media["readyState"].as_u64().unwrap_or(0) < 2
            || media["width"].as_u64().unwrap_or(0) == 0)
    {
        return Err(format!("Recorded preview failed: {media}"));
    }
    let mut border: u32 = 0;
    unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute(
            window.hwnd() as _,
            windows_sys::Win32::Graphics::Dwm::DWMWA_BORDER_COLOR as _,
            &mut border as *mut _ as _,
            4,
        );
    }
    capture(
        &window,
        root.join("blackbox-playback.png").to_str().unwrap(),
    )?;
    if window.has_undecorated_shadow() || guide.has_undecorated_shadow() {
        return Err("Native shadow re-enabled the top window border".into());
    }
    let layout = inspect(&window, r#"(()=>{const d=document.querySelector('iframe').contentDocument, b=[...d.querySelectorAll('.track-length-buttons button')].map(e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}});return {buttons:b,overflow:d.documentElement.scrollWidth>d.documentElement.clientWidth}})()"#).await?;
    if layout["overflow"] == true
        || layout["buttons"][0]["y"] != layout["buttons"][1]["y"]
        || layout["buttons"][0]["height"] != 30
    {
        return Err(format!("Blackbox controls layout regression: {layout}"));
    }
    for page in ["clips", "settings", "extract"] {
        inspect(
            &window,
            &format!("document.querySelector('.page-tab[data-page={page}]').click();true"),
        )
        .await?;
        tokio::time::sleep(Duration::from_millis(350)).await;
        capture(
            &window,
            root.join(format!("blackbox-{page}.png")).to_str().unwrap(),
        )?;
    }
    std::fs::write(root.join("window-audit.json"),serde_json::to_string_pretty(&json!({"guideCenterOffset":[dx,dy],"borderColor":border,"media":media,"recorder":status,"indexedChunks":session["segments"].as_array().map(|s|s.len()),"indexedSeconds":session["segments"].as_array().map(|s|s.iter().filter_map(|v|v["durationSeconds"].as_f64()).sum::<f64>()),"timelineSeconds":session["timelineDurationSeconds"]})).unwrap()).map_err(|e|e.to_string())?;
    Ok(())
}

async fn check_opacity_survives_show(
    window: &dioxus::desktop::DesktopContext,
) -> Result<(), String> {
    use dioxus::desktop::tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetLayeredWindowAttributes;
    let hwnd = window.hwnd() as _;
    let mut original: u8 = 255;
    unsafe {
        GetLayeredWindowAttributes(
            hwnd,
            std::ptr::null_mut(),
            &mut original,
            std::ptr::null_mut(),
        );
    }
    let topmost = window.is_always_on_top();
    for opacity in [0.0, 0.4] {
        crate::platform::set_opacity(window, opacity);
        window.set_visible(false);
        window.set_always_on_top(!topmost);
        window.set_visible(true);
        window.set_always_on_top(topmost);
        window.set_focus();
        tokio::time::sleep(Duration::from_millis(30)).await;
        let mut actual: u8 = 255;
        let valid = unsafe {
            GetLayeredWindowAttributes(
                hwnd,
                std::ptr::null_mut(),
                &mut actual,
                std::ptr::null_mut(),
            )
        };
        if valid == 0 || actual != (opacity * 255.0).round() as u8 {
            return Err(format!(
                "Window alpha lost on show/topmost/focus: opacity={opacity}, actual={actual}, valid={valid}"
            ));
        }
    }
    crate::platform::set_opacity(window, original as f64 / 255.0);
    Ok(())
}

// Exercise the real Win32 menu -> muda global handler -> Dioxus event route.
// Sending directly to the service would bypass the menu-routing regression.
fn activate_tray_menu(host: &Host, index: i32) -> Result<(), String> {
    use dioxus::desktop::{tao::platform::windows::WindowExtWindows, trayicon::menu::ContextMenu};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMenuItemID, SendMessageW, WM_COMMAND};
    let (menu, main) = {
        let host = host.0.borrow();
        (
            host.tray_menu.clone().ok_or("Native tray menu missing")?,
            host.main.clone(),
        )
    };
    unsafe {
        let command = GetMenuItemID(menu.hpopupmenu() as _, index);
        if command == u32::MAX {
            return Err("Native tray menu item missing".into());
        }
        let hwnd = main.hwnd();
        menu.attach_menu_subclass_for_hwnd(hwnd as isize);
        SendMessageW(hwnd as _, WM_COMMAND, command as usize, 0);
        menu.detach_menu_subclass_from_hwnd(hwnd as isize);
    }
    Ok(())
}

async fn escape_check(
    client: Client,
    host: Host,
    mut state: Signal<State>,
) -> Result<Value, String> {
    tokio::time::sleep(Duration::from_secs(3)).await;
    let main = host.0.borrow().main.clone();
    let mut results = vec![];
    let native_keys = std::env::args().any(|a| a == "--smoke-native-keys");
    let key = "(document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',code:'Escape',bubbles:true,cancelable:true})); true";
    for modal in [
        "close",
        "optimization",
        "network-confirm",
        "network-restore",
        "graphics-confirm",
        "reset",
        "turbo",
        "update",
        "input",
        "cpu",
    ] {
        state.write().modal = modal.into();
        state.write().tab = "advanced".into();
        tokio::time::sleep(Duration::from_millis(300)).await;
        if native_keys {
            native_escape(&main).await?;
        } else {
            inspect(&main, key).await?;
        }
        tokio::time::sleep(Duration::from_millis(450)).await;
        results.push(json!({"case":format!("main-{modal}"),"passed":state.peek().modal.is_empty() && state.peek().tab == "advanced"}));
    }
    for scenario in ["terms", "notice", "report", "busy"] {
        state.write().turbo["installed"] = json!(scenario != "terms");
        state.write().notice = if scenario == "notice" {
            json!({"id":"escape-test","markdown":"# Esc test"})
        } else {
            Value::Null
        };
        state.write().reports = if scenario == "report" {
            json!([{"responseId":"escape-test","title":"Esc test","message":"Test"}])
        } else {
            json!([])
        };
        state.write().modal = match scenario {
            "terms" => "turbo",
            "busy" => "reset",
            other => other,
        }
        .into();
        state.write().busy = scenario == "busy";
        tokio::time::sleep(Duration::from_millis(300)).await;
        if native_keys {
            native_escape(&main).await?;
        } else {
            inspect(&main, key).await?;
        }
        tokio::time::sleep(Duration::from_millis(450)).await;
        let passed = if scenario == "busy" {
            state.peek().modal == "reset"
        } else {
            state.peek().modal.is_empty()
        };
        results.push(json!({"case":format!("main-{scenario}"),"passed":passed}));
        state.write().busy = false;
    }
    state.write().modal = "reset".into();
    tokio::time::sleep(Duration::from_millis(250)).await;
    inspect(&main, &format!("{key}; (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown',{{key:'Escape',code:'Escape',repeat:true,bubbles:true,cancelable:true}})); true")).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    results.push(json!({"case":"main-repeat-preserves-underlying-panel","passed":state.peek().modal.is_empty() && state.peek().tab == "advanced"}));
    state.write().modal.clear();
    state.write().tab.clear();
    for (channel, scenario) in [
        ("character-guide", "normal"),
        ("dxvk-guide", "normal"),
        ("dxvk-manager", "normal"),
        ("blackbox-editor", "normal"),
        ("blackbox-manager", "normal"),
        ("blackbox-manager", "iframe"),
        ("blackbox-manager", "delete-modal"),
        ("blackbox-editor", "export-modal"),
        ("blackbox-manager", "iframe-export-modal"),
        ("blackbox-manager", "repeat"),
        ("blackbox-manager", "shortcut"),
    ] {
        client
            .invoke(1, &format!("application:open-{channel}"), json!([]))
            .await?;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let (id, child) = host
            .0
            .borrow()
            .windows
            .iter()
            .find(|(id, _)| **id != 1)
            .map(|(id, w)| (*id, w.clone()))
            .ok_or("Auxiliary window missing")?;
        let (setup, target, preserved, selector) = match scenario {
            "iframe" => (
                "",
                "document.querySelector('iframe').contentDocument.body",
                false,
                "",
            ),
            "delete-modal" => (
                "document.querySelector('.clip-delete-modal').hidden=false;",
                "document.body",
                true,
                ".clip-delete-modal",
            ),
            "export-modal" => (
                "document.querySelector('.export-modal').hidden=false;",
                "document.body",
                true,
                ".export-modal",
            ),
            "iframe-export-modal" => (
                "document.querySelector('iframe').contentDocument.querySelector('.export-modal').hidden=false;",
                "document.querySelector('iframe').contentDocument.body",
                true,
                ".export-modal",
            ),
            "shortcut" => (
                "document.querySelector('[data-page=settings].page-tab').click();document.querySelector('.shortcut-input').focus();",
                "document.querySelector('.shortcut-input')",
                true,
                "",
            ),
            "repeat" => ("", "document.body", true, ""),
            _ => ("", "document.body", false, ""),
        };
        if native_keys && scenario != "repeat" {
            inspect(
                &child,
                &format!("{setup} {target}.setAttribute('tabindex','-1'); {target}.focus();true"),
            )
            .await?;
            native_escape(&child).await.map_err(|e|format!("{channel}-{scenario}: {e}"))?;
        } else {
            inspect(&child, &format!("{setup} setTimeout(()=>{target}.dispatchEvent(new KeyboardEvent('keydown',{{key:'Escape',code:'Escape',repeat:{},bubbles:true,cancelable:true}})),50);true", scenario == "repeat")).await?;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let exists = host.0.borrow().windows.contains_key(&id);
        let modal_closed = if exists && !selector.is_empty() {
            let doc = if scenario == "iframe-export-modal" {
                "document.querySelector('iframe').contentDocument"
            } else {
                "document"
            };
            inspect(&child, &format!("{doc}.querySelector('{selector}').hidden")).await? == true
        } else {
            selector.is_empty()
        };
        let shortcut_disabled = if exists && scenario == "shortcut" {
            inspect(&child, "document.querySelector('.shortcut-input').value === '사용 안 함' && document.activeElement !== document.querySelector('.shortcut-input')").await? == true
        } else {
            true
        };
        results.push(json!({"case":format!("{channel}-{scenario}"),"passed":exists == preserved && modal_closed && shortcut_disabled,"windowPreserved":exists,"modalClosed":modal_closed}));
        if exists {
            inspect(&child, "setTimeout(()=>window.close(),50);true").await?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    Ok(
        json!({"passed":results.iter().all(|r| r["passed"] == true),"input":if native_keys { "Windows WM_KEYDOWN/WM_KEYUP to focused WebView HWND (repeat cases use DOM KeyboardEvent)" } else { "WebView2 DOM KeyboardEvent" },"cases":results}),
    )
}

#[cfg(target_os = "windows")]
async fn native_escape(window: &dioxus::desktop::DesktopContext) -> Result<(), String> {
    use dioxus::desktop::tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    window.set_visible(true);
    let root = window.hwnd() as _;
    let thread = unsafe { GetWindowThreadProcessId(root, std::ptr::null_mut()) };
    let mut info: GUITHREADINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    let mut ready=false;
    for _ in 0..10 {
        window.set_focus();
        window.webview.focus().map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        ready=unsafe {GetGUIThreadInfo(thread,&mut info)!=0 && !info.hwndFocus.is_null() && (info.hwndFocus==root || IsChild(root,info.hwndFocus)!=0)};
        if ready {break;}
    }
    if !ready {return Err("No focused child in the test window".into());}
    unsafe {
        // Address only this WebView's focus HWND; never send input to another app.
        if PostMessageW(info.hwndFocus, WM_KEYDOWN, 0x1b, 1 | (1 << 16)) == 0
            || PostMessageW(
                info.hwndFocus,
                WM_KEYUP,
                0x1b,
                (1_u32 | (1 << 16) | (1 << 30) | (1 << 31)) as isize,
            ) == 0
        {
            return Err("Windows Escape message injection failed".into());
        }
    }
    Ok(())
}

async fn boost_mask_check(client: Client, host: Host, mut state: Signal<State>) -> Result<Value,String> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main=host.0.borrow().main.clone();
    main.set_focus();
    state.write().visual_active=true;
    state.write().document_hidden=false;
    let mut cases=vec![];
    for paused in [true,false,true,false] {
        // Only fixture state is altered; no real optimizer is invoked.
        state.write().services["affinity"]["gameActive"]=json!(true);
        tokio::time::sleep(Duration::from_millis(50)).await;
        inspect(&main,"document.querySelector('.boost-text-area').dispatchEvent(new MouseEvent('click',{bubbles:true,clientX:320,clientY:216}));true").await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let during=inspect(&main,"({wave:window.nogiremWave.inspect(),overlay:!!document.querySelector('.boost-color-overlay'),basePaused:document.querySelector('.boost-home').classList.contains('paused'),copy:document.querySelector('.boost-text-area').textContent,style:document.querySelector('.boost-color-overlay')?.getAttribute('style')})").await?;
        if during["wave"]["transition"]["targetDark"]!=paused || during["basePaused"]!=!paused || during["overlay"]!=true {
            return Err(format!("Action mask canceled/reversed while backend pending: {during}"));
        }
        if paused && !during["copy"].as_str().unwrap_or("").contains("부스트 중단중") {return Err(format!("Missing stopping status: {during}"));}
        // Unrelated state updates and service polling must not reset the mask.
        state.write().notices.insert("mask-probe".into(),"updated".into());
        tokio::time::sleep(Duration::from_millis(if paused {150}else{900})).await;
        let mid=inspect(&main,"({wave:window.nogiremWave.inspect(),clip:getComputedStyle(document.querySelector('.boost-color-overlay')).clipPath})").await?;
        if mid["wave"]["transition"]["targetDark"]!=paused {return Err(format!("Mask lost during state update: {mid}"));}
        let duration=if paused {600}else{3000};
        if mid["wave"]["transition"]["duration"]!=duration {return Err(format!("Original timing changed: {mid}"));}
        tokio::time::sleep(Duration::from_millis(if paused {700}else{2850})).await;
        let end=inspect(&main,"({wave:window.nogiremWave.inspect(),overlay:!!document.querySelector('.boost-color-overlay'),basePaused:document.querySelector('.boost-home').classList.contains('paused')})").await?;
        if end["overlay"]!=false || end["basePaused"]!=paused || end["wave"]["darkBackground"]!=paused || !end["wave"]["transition"].is_null() {return Err(format!("Mask completion mismatch: {end}"));}
        cases.push(json!({"paused":paused,"during":during,"middle":mid,"end":end}));
    }
    client.invoke(1,"smoke:fail-next-boost",json!([])).await?;
    state.write().services["affinity"]["gameActive"]=json!(true);
    tokio::time::sleep(Duration::from_millis(50)).await;
    inspect(&main,"document.querySelector('.boost-text-area').dispatchEvent(new MouseEvent('click',{bubbles:true,clientX:320,clientY:216}));true").await?;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let rollback=inspect(&main,"({wave:window.nogiremWave.inspect(),overlay:!!document.querySelector('.boost-color-overlay'),basePaused:document.querySelector('.boost-home').classList.contains('paused')})").await?;
    if rollback["overlay"]!=false || rollback["basePaused"]!=false || rollback["wave"]["darkBackground"]!=false || !rollback["wave"]["transition"].is_null(){return Err(format!("Failed action did not restore original color: {rollback}"));}
    state.write().error.clear();
    Ok(json!({"passed":true,"cases":cases,"failureRollback":rollback}))
}

async fn terms_scroll_check(host:Host,mut state:Signal<State>)->Result<Value,String>{
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main=host.0.borrow().main.clone();
    state.write().turbo["installed"]=json!(false);
    state.write().modal="turbo".into();
    tokio::time::sleep(Duration::from_millis(400)).await;
    let layout=inspect(&main,r#"(()=>{const e=document.querySelector('.terms-scroll'),f=document.querySelector('.terms-modal>footer'),h=document.querySelector('.terms-modal>header'),r=e.getBoundingClientRect();return{top:r.top,bottom:r.bottom,headerBottom:h.getBoundingClientRect().bottom,footerTop:f.getBoundingClientRect().top,overflowX:e.scrollWidth-e.clientWidth,scheme:getComputedStyle(e).colorScheme,width:getComputedStyle(e).scrollbarWidth,maximum:e.scrollHeight-e.clientHeight,connected:!!window.nogiremTermsScroller}})()"#).await?;
    if layout["connected"]!=true || layout["scheme"]!="dark" || layout["width"]!="thin" || layout["top"]!=layout["headerBottom"] || layout["bottom"]!=layout["footerTop"] || layout["overflowX"].as_f64().unwrap_or(1.)>0. {return Err(format!("Terms scroll layout: {layout}"));}
    let wheel=inspect(&main,r#"(()=>{const e=document.querySelector('.terms-scroll'),event=new WheelEvent('wheel',{deltaY:120,bubbles:true,cancelable:true});e.dispatchEvent(event);return{prevented:event.defaultPrevented,immediate:e.scrollTop}})()"#).await?;
    tokio::time::sleep(Duration::from_millis(70)).await;
    let middle=inspect(&main,"document.querySelector('.terms-scroll').scrollTop").await?;
    tokio::time::sleep(Duration::from_millis(700)).await;
    let settled=inspect(&main,"document.querySelector('.terms-scroll').scrollTop").await?;
    if wheel["prevented"]!=true || middle.as_f64().unwrap_or(0.)<=0. || middle.as_f64()>=settled.as_f64(){return Err(format!("Wheel interpolation failed: {wheel}, {middle}, {settled}"));}
    inspect(&main,"const e=document.querySelector('.terms-scroll');e.scrollTop=e.scrollHeight;e.dispatchEvent(new WheelEvent('wheel',{deltaY:120,bubbles:true,cancelable:true}));true").await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let bottom=inspect(&main,"(()=>{const e=document.querySelector('.terms-scroll');return Math.abs(e.scrollTop-(e.scrollHeight-e.clientHeight))<1})()").await?;
    if bottom!=true{return Err("Terms cannot reach bottom".into());}
    if let Some(report)=std::env::args().find_map(|a|a.strip_prefix("--smoke-report=").map(str::to_owned)){capture(&main,&report.replace(".json",".png"))?;}
    state.write().modal.clear();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let disposed=inspect(&main,"!window.nogiremTermsScroller").await?;
    state.write().modal="turbo".into();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let reopened=inspect(&main,"document.querySelector('.terms-scroll').scrollTop").await?;
    if disposed!=true || reopened!=0 {return Err(format!("Terms scroll cleanup/reopen failed: {disposed}, {reopened}"));}
    state.write().modal.clear();
    Ok(json!({"passed":true,"layout":layout,"wheel":wheel,"middle":middle,"settled":settled,"bottomReached":bottom,"disposed":disposed,"reopened":reopened}))
}

async fn announcements_check(host:Host,mut state:Signal<State>)->Result<Value,String>{
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main=host.0.borrow().main.clone();
    {let mut s=state.write();s.modal.clear();s.notice=Value::Null;s.reports=json!([]);s.seen_report_ids.clear();}
    crate::ui::receive(state,"application:notice-available",&json!({"id":"a".repeat(64),"markdown":include_str!("../../NOTICE.md")}));
    tokio::time::sleep(Duration::from_millis(350)).await;
    let notice=inspect(&main,"({title:document.querySelector('.application-notice-title')?.textContent,headers:document.querySelectorAll('.markdown-table th').length,cells:document.querySelectorAll('.markdown-table td').length,overflow:document.querySelector('.notice-content').scrollWidth-document.querySelector('.notice-content').clientWidth})").await?;
    if notice["headers"]!=4 || notice["cells"]!=16 || notice["overflow"].as_f64().unwrap_or(1.)>1. {return Err(format!("Notice table: {notice}"));}
    if let Some(report)=std::env::args().find_map(|a|a.strip_prefix("--smoke-report=").map(str::to_owned)){capture(&main,&report.replace(".json",".png"))?;}
    let notice_scroll = notice_scroll_check(&main).await?;
    let first=json!({"responseId":"first","reportId":"fixture","title":"첫 답변","message":"긴 리포트 답변 스크롤 확인입니다. ".repeat(100),"answeredAt":"2026-09-29T00:00:00Z"});
    let second=json!({"responseId":"second","reportId":"fixture","title":"두 번째 답변","message":"두 번째 본문","answeredAt":"2026-09-29T00:00:00Z"});
    crate::ui::receive(state,"application:report-responses-available",&json!([first,second]));
    crate::ui::receive(state,"application:report-responses-available",&json!([]));
    crate::ui::receive(state,"application:report-responses-available",&json!([first]));
    tokio::time::sleep(Duration::from_millis(350)).await;
    if inspect(&main,"document.querySelector('.report-response-content h2')?.textContent").await?!="첫 답변" {return Err("First report lost".into());}
    let report_scroll = notice_scroll_check(&main).await?;
    inspect(&main,"document.querySelector('.report-response-confirm').click();true").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if inspect(&main,"document.querySelector('.report-response-content h2')?.textContent").await?!="두 번째 답변" {return Err("Second report lost".into());}
    if inspect(&main,"document.querySelector('.notice-content').scrollTop").await?!=0 {return Err("Next reply retained old scroll position".into());}
    inspect(&main,"document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));true").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if inspect(&main,"!!document.querySelector('.application-notice-content')").await?!=true {return Err("Notice not restored after reports".into());}
    inspect(&main,"document.querySelector('.application-notice-confirm').click();true").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if inspect(&main,"!!document.querySelector('.notice-dialog')").await?!=false {return Err("Notice confirmation did not close".into());}
    if inspect(&main,"!window.nogiremNoticeScroller").await?!=true {return Err("Notice scroller not disposed".into());}
    crate::ui::receive(state,"application:notice-available",&json!({"id":"b".repeat(64),"markdown":include_str!("../../NOTICE.md")}));
    tokio::time::sleep(Duration::from_millis(300)).await;
    if inspect(&main,"document.querySelector('.notice-content').scrollTop===0 && !!window.nogiremNoticeScroller").await?!=true {return Err("Reopened notice scroller not reset".into());}
    state.write().modal.clear();
    Ok(json!({"passed":true,"notice":notice,"noticeScroll":notice_scroll,"reportScroll":report_scroll,"cleanup":true,"reopen":true,"reportQueue":true,"duplicateDelivery":true,"emptyRecheck":true,"confirm":true,"escape":true,"noticeAfterReports":true}))
}

// Interactive preview uses the production notice renderer with isolated fixture services.
pub async fn preview_notice(host:Host,mut state:Signal<State>,path:&str)->Result<Value,String>{
    for _ in 0..100 {
        if state.peek().ready { break; }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let markdown=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
    let notice=nogirem_backend::app_services::normalize_notice(&markdown)?;
    { let mut s=state.write();s.modal.clear();s.reports=json!([]); }
    crate::ui::receive(state,"application:notice-available",&notice);
    let main=host.0.borrow().main.clone();
    main.window.set_visible(true);main.window.set_focus();
    tokio::time::sleep(Duration::from_millis(400)).await;
    let view=inspect(&main,"({title:document.querySelector('.application-notice-title')?.textContent,tableRows:document.querySelectorAll('.markdown-table tbody tr').length,overflow:document.querySelector('.notice-content').scrollWidth-document.querySelector('.notice-content').clientWidth,text:document.querySelector('.application-notice-content')?.innerText})").await?;
    if let Some(report)=std::env::args().find_map(|a|a.strip_prefix("--preview-report=").map(str::to_owned)) {
        capture(&main,&report.replace(".json",".png"))?;
    }
    Ok(json!({"passed":true,"source":path,"rendered":view,"leftOpen":true}))
}

async fn notice_scroll_check(main:&dioxus::desktop::DesktopContext)->Result<Value,String>{
    let initial=inspect(main,"(()=>{const e=document.querySelector('.notice-content');return {connected:!!window.nogiremNoticeScroller,top:e.scrollTop,maximum:e.scrollHeight-e.clientHeight}})()").await?;
    if initial["connected"]!=true || initial["top"]!=0 || initial["maximum"].as_f64().unwrap_or(0.)<=0. {return Err(format!("Notice scroll setup: {initial}"));}
    let wheel=inspect(main,"(()=>{const e=document.querySelector('.notice-content'),w=new WheelEvent('wheel',{deltaY:120,bubbles:true,cancelable:true});e.dispatchEvent(w);return {prevented:w.defaultPrevented,immediate:e.scrollTop}})()").await?;
    tokio::time::sleep(Duration::from_millis(40)).await;
    let middle=inspect(main,"document.querySelector('.notice-content').scrollTop").await?;
    tokio::time::sleep(Duration::from_millis(700)).await;
    let end=inspect(main,"document.querySelector('.notice-content').scrollTop").await?;
    if wheel["prevented"]!=true || wheel["immediate"]!=0 || middle.as_f64().unwrap_or(0.)<=0. || middle.as_f64()>=end.as_f64(){return Err(format!("Notice easing: {wheel} {middle} {end}"));}
    inspect(main,"(()=>{const e=document.querySelector('.notice-content');e.dispatchEvent(new PointerEvent('pointerdown'));e.scrollTop=e.scrollHeight;e.dispatchEvent(new WheelEvent('wheel',{deltaY:120,bubbles:true,cancelable:true}));return true})()").await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    if inspect(main,"(()=>{const e=document.querySelector('.notice-content');return Math.abs(e.scrollTop-(e.scrollHeight-e.clientHeight))<=1})()").await?!=true{return Err(format!("Notice bottom boundary failed: {}",inspect(main,"(()=>{const e=document.querySelector('.notice-content');return {top:e.scrollTop,max:e.scrollHeight-e.clientHeight,key:window.nogiremNoticeScroller?.key}})()").await?));}
    Ok(json!({"initial":initial,"wheel":wheel,"middle":middle,"settled":end,"bottomReached":true}))
}

async fn updater_check(client:Client,host:Host,mut state:Signal<State>)->Result<Value,String>{
    tokio::time::sleep(Duration::from_secs(6)).await;
    let main=host.0.borrow().main.clone();state.write().modal.clear();
    let existing:Vec<_>=host.0.borrow().windows.keys().copied().collect();
    use dioxus::desktop::tao::platform::windows::WindowExtWindows;
    main.set_focus();tokio::time::sleep(Duration::from_millis(200)).await;
    let foreground=unsafe{windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow()};
    client.invoke(1,"smoke:update-state",json!([{"phase":"available","version":"0.4.1","percent":0}])).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let toast_id=host.0.borrow().windows.keys().find(|id|!existing.contains(id)).copied().ok_or("Update notification did not open")?;
    let toast=host.0.borrow().windows[&toast_id].clone();
    let toast_dom=inspect(&toast,r#"({text:document.querySelector('.notice').innerText,color:getComputedStyle(document.querySelector('.notice')).backgroundColor,width:innerWidth,height:innerHeight})"#).await?;
    if toast_dom["text"]!="마비노기 렘 부스터 새 버전 업데이트가 가능합니다" || toast_dom["color"]!="rgb(255, 157, 0)" || toast_dom["height"]!=88{return Err(format!("Toast parity: {toast_dom}"));}
    if unsafe{windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow()}!=foreground{return Err(format!("Update notification stole focus: before={foreground:?}, after={:?}, toast={:?}, main={:?}",unsafe{windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow()},toast.hwnd(),main.hwnd()));}
    let position=toast.outer_position().map_err(|e|e.to_string())?.to_logical::<f64>(toast.scale_factor());
    let toast_position=json!({"x":position.x,"y":position.y});
    let native_style=unsafe{windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongW(toast.hwnd() as _,windows_sys::Win32::UI::WindowsAndMessaging::GWL_STYLE)} as u32;
    if native_style & windows_sys::Win32::UI::WindowsAndMessaging::WS_CAPTION != 0 {return Err(format!("Notification native title bar: {native_style:#x}"));}
    if let Some(prefix)=std::env::args().find_map(|a|a.strip_prefix("--capture=").map(str::to_owned)){capture(&toast,&format!("{prefix}-notification.png"))?;}

    client.invoke(1,"smoke:update-state",json!([{"phase":"available","version":"0.4.1","percent":0}])).await?;
    if !host.0.borrow().windows.contains_key(&toast_id) {return Err("Duplicate notification recreated the window".into());}
    let badge=inspect(&main,r#"({text:document.querySelector('.app-version').textContent,highlight:document.querySelector('.app-version').classList.contains('update-available')})"#).await?;
    if badge["text"]!="새 버전 출시됨" || badge["highlight"]!=true {return Err(format!("Update badge parity: {badge}"));}
    inspect(&main,"(()=>{document.querySelector('.app-version').click();return true})()").await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let started=inspect(&main,"document.querySelector('.update-preview-panel')?.innerText").await?;
    if !started.as_str().unwrap_or("").contains("새 버전을 가져오고 있습니다"){return Err(format!("Version click did not start download: {started}"));}
    client.invoke(1,"smoke:update-state",json!([{"phase":"downloading","version":"0.4.1","percent":42.9}])).await?;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let progress=inspect(&main,r#"(()=>{const p=document.querySelector('.update-preview-panel'),r=p.getBoundingClientRect();return {text:p.innerText,top:r.top,height:r.height,color:getComputedStyle(p).backgroundColor,bar:document.querySelector('.update-progress-value').style.width,modal:!!document.querySelector('.modal-sheet')}})()"#).await?;
    if progress["top"]!=64 || progress["height"]!=156 || progress["color"]!="rgb(255, 157, 0)" || progress["bar"]!="42.9%" || progress["modal"]!=false || !progress["text"].as_str().unwrap_or("").contains("42%") {return Err(format!("Download panel parity: {progress}"));}
    if let Some(prefix)=std::env::args().find_map(|a|a.strip_prefix("--capture=").map(str::to_owned)){capture(&main,&format!("{prefix}-progress.png"))?;}
    let acks_before=client.invoke(1,"smoke:announcement-acks",json!([])).await?;
    state.write().notice=json!({"id":"update-unread-test","markdown":"# 업데이트 공지\n확인하기 전까지 읽지 않은 공지입니다."});state.write().modal="notice".into();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let foreground_notice=inspect(&main,"!!document.elementFromPoint(320,120)?.closest('.notice-backdrop')").await?;
    if foreground_notice!=true {return Err("Update panel covered the unread notice".into());}
    client.invoke(1,"smoke:update-state",json!([{"phase":"downloaded","version":"0.4.1","percent":100}])).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    state.write().modal.clear(); // Unmount as during shutdown, without a user's confirmation.
    tokio::time::sleep(Duration::from_millis(100)).await;
    if client.invoke(1,"smoke:announcement-acks",json!([])).await?!=acks_before{return Err("Update/unmount acknowledged an unread notice".into());}
    state.write().modal="notice".into();tokio::time::sleep(Duration::from_millis(150)).await;
    if inspect(&main,"!!document.querySelector('.notice-dialog')").await?!=true{return Err("Unread notice was lost".into());}
    inspect(&main,"(()=>{document.querySelector('.application-notice-confirm').click();return true})()").await?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    let acks_after=client.invoke(1,"smoke:announcement-acks",json!([])).await?;
    if acks_after["notice"].as_u64()!=Some(acks_before["notice"].as_u64().unwrap_or(0)+1){return Err("Explicit notice confirmation did not acknowledge".into());}
    state.write().modal="close".into();tokio::time::sleep(Duration::from_millis(100)).await;
    if inspect(&main,"!!document.querySelector('.update-preview-overlay')").await?!=false{return Err("Close confirmation must hide the update panel".into());}
    state.write().modal.clear();
    client.invoke(1,"smoke:update-state",json!([{"phase":"downloaded","version":"0.4.1","percent":100}])).await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let downloaded=inspect(&main,"document.querySelector('.update-preview-panel').innerText").await?;
    if !downloaded.as_str().unwrap_or("").contains("새 버전 설치") {return Err(format!("Install button parity: {downloaded}"));}
    inspect(&main,"(()=>{document.querySelector('.update-preview-panel button').click();return true})()").await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    if inspect(&main,"document.querySelector('.update-preview-panel button').disabled").await?!=true {return Err("Repeated install must be disabled".into());}
    client.invoke(1,"smoke:update-state",json!([{"phase":"error","error":"서명 검증 실패"}])).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    if inspect(&main,"!!document.querySelector('.update-preview-overlay')").await?!=false{return Err("Failed download left a blocking panel".into());}
    tokio::time::sleep(Duration::from_secs(5)).await;
    if host.0.borrow().windows.contains_key(&toast_id){return Err("Notification did not close after 4.8 seconds".into());}
    client.invoke(1,"smoke:update-state",json!([{"phase":"available","version":"0.4.1","percent":0}])).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    if host.0.borrow().windows.keys().any(|id|!existing.contains(id)){return Err("Same version notified twice in one session".into());}
    client.invoke(1,"smoke:update-state",json!([{"phase":"idle"}])).await?;
    Ok(json!({"passed":true,"badge":badge,"notification":toast_dom,"notificationPosition":toast_position,"noFocusSteal":true,"deduplicated":true,"autoDismissed":true,"progress":progress,"downloaded":downloaded,"nativeFrameAbsent":true,"noticeAboveUpdate":true,"unreadPreservedWithoutConfirmation":true,"installerExecuted":false}))
}

async fn creator_prompt_check(client:Client,host:Host,mut state:Signal<State>)->Result<Value,String>{
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main=host.0.borrow().main.clone();main.set_focus();tokio::time::sleep(Duration::from_millis(200)).await;
    client.invoke(1,"smoke:creator-prompt-count",json!([0])).await?;
    {let mut s=state.write();s.creator_prompt=true;s.creator_prompt_loaded=true;s.navigation_ready=false;s.controls_entered=false;s.identity="transition".into();s.visual_active=true;s.document_hidden=false;s.modal.clear();s.tab.clear();s.creator_phase="home".into();}
    crate::wave::finish_identity(state);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let opening=inspect(&main,"({opacity:getComputedStyle(document.querySelector('.creator-credit')).opacity,disabled:document.querySelector('.creator-credit').disabled,prompt:!!document.querySelector('.creator-prompt')})").await?;
    if opening["opacity"]!="0" || opening["disabled"]!=true || opening["prompt"]!=false{return Err(format!("Creator shown during identity: {opening}"));}
    tokio::time::sleep(Duration::from_millis(1750)).await;
    if state.peek().navigation_ready {return Err("Creator navigation advanced before its 3s delay".into());}
    tokio::time::sleep(Duration::from_millis(3400)).await;
    let shown=inspect(&main,"({opacity:getComputedStyle(document.querySelector('.creator-credit')).opacity,disabled:document.querySelector('.creator-credit').disabled,prompt:document.querySelector('.creator-prompt')?.textContent})").await?;
    if shown["opacity"]!="1" || shown["disabled"]!=false || shown["prompt"]!="고급 기능은 여기에서" {return Err(format!("Creator ready: {shown}; eligible={}, loaded={}, visual={}, hidden={}, phase={}, tab={}, modal={}, identity={}, interface={}",state.peek().creator_prompt,state.peek().creator_prompt_loaded,state.peek().visual_active,state.peek().document_hidden,state.peek().creator_phase,state.peek().tab,state.peek().modal,state.peek().identity,state.peek().interface_visible));}
    state.write().visual_active=false;tokio::time::sleep(Duration::from_millis(250)).await;
    if inspect(&main,"!!document.querySelector('.creator-prompt')").await?!=false{return Err("Inactive window retained prompt".into());}
    state.write().visual_active=true;tokio::time::sleep(Duration::from_millis(350)).await;
    state.write().modal="input".into();tokio::time::sleep(Duration::from_millis(250)).await;
    if inspect(&main,"!!document.querySelector('.creator-prompt')").await?!=false{return Err("Settings retained prompt".into());}
    state.write().modal.clear();tokio::time::sleep(Duration::from_millis(350)).await;
    let untouched=client.invoke(1,"smoke:creator-prompt-state",json!([])).await?;
    if untouched["count"]!=0 || untouched["displayCalls"]!=0 {return Err(format!("Displaying tip changed count: {untouched}"));}
    for count in 1..=2 {
        inspect(&main,"(()=>{document.querySelector('.creator-credit').click();return true})()").await?;
        tokio::time::sleep(Duration::from_millis(900)).await;
        let saved=client.invoke(1,"smoke:creator-prompt-state",json!([])).await?;
        if state.peek().tab!="developer" || state.peek().creator_prompt || saved["count"]!=count{return Err(format!("Creator opening count: {saved}"));}
        inspect(&main,"(()=>{document.querySelector('.creator-credit').click();return true})()").await?;
        tokio::time::sleep(Duration::from_millis(900)).await;
        if client.invoke(1,"smoke:creator-prompt-state",json!([])).await?["count"]!=count{return Err("Returning home counted as opening".into());}
        if inspect(&main,"!!document.querySelector('.creator-prompt')").await?!=false{return Err("Dismissed tip reappeared in same session".into());}
        // Re-read stored preference as a subsequent launch would.
        let dismissed=client.invoke(1,"application:get-creator-prompt-dismissed",json!([])).await?;
        state.write().creator_prompt=dismissed!=true;tokio::time::sleep(Duration::from_millis(350)).await;
        if inspect(&main,"!!document.querySelector('.creator-prompt')").await?!=json!(count<2){return Err("Two-click cutoff not preserved on preference reload".into());}
    }
    Ok(json!({"passed":true,"opening":opening,"ready":shown,"displayCountUnchanged":true,"onlyOpeningClicksCount":true,"twoClickLimit":true,"timing":"identity done + 3000ms"}))
}

async fn visual_activity_check(host:Host,mut state:Signal<State>)->Result<Value,String>{
    tokio::time::sleep(Duration::from_secs(2)).await;
    let main=host.0.borrow().main.clone();
    {let mut s=state.write();s.visual_active=true;s.document_hidden=false;s.identity="done".into();s.tab.clear();s.modal.clear();s.services["affinity"]["running"]=json!(true);s.services["memory"]["running"]=json!(true);s.services["affinity"]["gameActive"]=json!(true);s.services["memory"]["gameActive"]=json!(true);}
    tokio::time::sleep(Duration::from_secs(3)).await;
    let script="({playState:getComputedStyle(document.querySelector('.boost-text-final')).animationPlayState,wave:window.nogiremWave.inspect()})";
    let active=inspect(&main,script).await?;
    if active["playState"]!="running" || active["wave"]["pageVisible"]!=true{return Err(format!("Active visuals missing: {active}"));}
    for hidden in [false,true] {
        {let mut s=state.write();s.visual_active=hidden;s.document_hidden=hidden;}
        tokio::time::sleep(Duration::from_millis(250)).await;
        let before=inspect(&main,script).await?;
        // Background service updates and mouse movement must not restart drawing.
        inspect(&main,"(()=>{window.dispatchEvent(new MouseEvent('mousemove',{clientX:600,clientY:250}));window.nogiremApplyWave();return true})()").await?;
        tokio::time::sleep(Duration::from_millis(1800)).await;
        let after=inspect(&main,script).await?;
        if after["playState"]!="paused" || after["wave"]["pageVisible"]!=false || before["wave"]["renderedFrames"]!=after["wave"]["renderedFrames"] || after["wave"]["framePending"]!=false || after["wave"]["ambientPending"]!=false{return Err(format!("Inactive visuals still running (hidden={hidden}): {before} -> {after}"));}
        {let mut s=state.write();s.visual_active=true;s.document_hidden=false;}
        tokio::time::sleep(Duration::from_millis(1600)).await;
        let resumed=inspect(&main,script).await?;
        if resumed["playState"]!="running" || resumed["wave"]["pageVisible"]!=true || resumed["wave"]["renderedFrames"].as_u64()<=after["wave"]["renderedFrames"].as_u64(){return Err(format!("Visuals failed to resume: {resumed}"));}
    }
    Ok(json!({"passed":true,"blurPausesPulseAndCanvas":true,"documentHiddenPauses":true,"backgroundUpdatesStayIdle":true,"focusResumes":true}))
}
