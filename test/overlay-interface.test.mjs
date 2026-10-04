import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"

const [applicationView, backend, service, windows, native, overlay, preload, packager, styles] = await Promise.all([
  readFile(new URL("../desktop/src/ui.rs", import.meta.url), "utf8"),
  readFile(new URL("../desktop/backend/src/channel_ping.rs", import.meta.url), "utf8"),
  readFile(new URL("../desktop/backend/src/service.rs", import.meta.url), "utf8"),
  readFile(new URL("../desktop/backend/src/service_windows.rs", import.meta.url), "utf8"),
  readFile(new URL("../desktop/src/native.rs", import.meta.url), "utf8"),
  readFile(new URL("../channel-ping-overlay.html", import.meta.url), "utf8"),
  readFile(new URL("../service/channel-ping-overlay-preload.js", import.meta.url), "utf8"),
  readFile(new URL("../scripts/package-dioxus.mjs", import.meta.url), "utf8"),
  readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
])

test("고급 기능에서 채널별 핑 오버레이를 켜고 끈다", () => {
  assert.match(applicationView, /h2 \{ "채널별 핑" \}/)
  assert.match(applicationView, /application:set-channel-ping-setting/)
  assert.match(backend, /REMOTE_URL[\s\S]+channel\.csv/)
  assert.match(backend, /MEASURE_INTERVAL[\s\S]+from_secs\(60\)/)
  assert.match(backend, /FAILURE_SYNC_INTERVAL[\s\S]+60 \* 60/)
  assert.match(backend, /WH_KEYBOARD_LL/)
  assert.match(backend, /WH_MOUSE_LL/)
  assert.doesNotMatch(backend, /GetAsyncKeyState/)
  assert.match(service, /"channel-ping" => kind == "channel-ping-overlay"/)
  assert.match(service, /prepare_channel_ping\(\)[\s\S]+InputMonitor::new\(\)/)
  assert.match(service, /let mut overlay_visible = false/)
  assert.match(service, /sync_overlay_window\([\s\S]+channel_ping_id\(\)/)
  assert.match(service, /\.poll\(overlay_visible, &self\.env\)/)
  assert.match(applicationView, /application:get-channel-ping-setting/)
})

test("채널별 핑 창은 클릭 통과 비활성 topmost 창으로 패키징된다", () => {
  assert.match(windows, /"channel-ping-overlay"[\s\S]+transparent/)
  assert.match(windows, /"channel-ping-overlay"[\s\S]+width":584/)
  assert.match(backend, /let width = 584\.0/)
  assert.match(windows, /"channel-ping-overlay"[\s\S]+alwaysOnTop/)
  assert.match(windows, /ignoreMouseEvents/)
  assert.match(backend, /ShowWindowAsync/)
  assert.match(windows, /"opacity", json!\(1\)/)
  assert.match(windows, /physicalBounds/)
  assert.match(windows, /"showInactive"[\s\S]+rpc\.request/)
  assert.match(windows, /채널 핑 오버레이가 준비되지 않았습니다/)
  assert.match(native, /"physicalBounds"/)
  assert.match(native, /"channel-ping-overlay\.html"/)
  assert.match(native, /"hwnd":window\.hwnd\(\) as usize/)
  assert.match(overlay, /키보드 또는 마우스를 누르면 닫힙니다/)
  assert.match(overlay, /const channelGroups = \[\[1, 15\], \[16, 29\], \[30, 38\]\]/)
  assert.match(overlay, /grid-template-columns:\s*repeat\(3, 170px\)/)
  assert.match(overlay, /justify-content:\s*space-between/)
  assert.doesNotMatch(overlay, /0 18px 50px/)
  assert.match(overlay, /logo2-white-transparent\.png/)
  assert.match(overlay, /초 전에 측정했습니다/)
  assert.match(overlay, /value < 5[\s\S]+value < 10[\s\S]+value < 15/)
  assert.match(overlay, /className = `\$\{grade[\s\S]+best/)
  assert.match(preload, /channel-ping:get-status/)
  assert.doesNotMatch(preload, /set|write|open/)
  assert.match(packager, /channel-ping-overlay\.html/)
  assert.match(packager, /channel\.csv/)
})

test("정지 화면의 터보키 제거 버튼은 문구 양옆 여백을 유지한다", () => {
  assert.match(styles, /button\.turbo-key-remove\s*\{[\s\S]*min-width:\s*72px/)
  assert.match(styles, /button\.turbo-key-remove\s*\{[\s\S]*padding:\s*0 5px/)
})
