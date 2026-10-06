import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"

const root = new URL("../", import.meta.url)
const [
  settings,
  preview,
  alert,
  configuration,
  settingsPreload,
  previewPreload,
  alertPreload,
  configurationPreload,
  service,
  windows,
  native,
  packager,
  ui,
  contract,
] = await Promise.all([
  readFile(new URL("overlay-settings.html", root), "utf8"),
  readFile(new URL("debuff-region-overlay.html", root), "utf8"),
  readFile(new URL("debuff-alert-overlay.html", root), "utf8"),
  readFile(new URL("overlay-configuration.html", root), "utf8"),
  readFile(new URL("service/overlay-settings-preload.js", root), "utf8"),
  readFile(new URL("service/debuff-region-overlay-preload.js", root), "utf8"),
  readFile(new URL("service/debuff-alert-overlay-preload.js", root), "utf8"),
  readFile(new URL("service/overlay-configuration-preload.js", root), "utf8"),
  readFile(new URL("desktop/backend/src/service.rs", root), "utf8"),
  readFile(new URL("desktop/backend/src/service_windows.rs", root), "utf8"),
  readFile(new URL("desktop/src/native.rs", root), "utf8"),
  readFile(new URL("scripts/package-dioxus.mjs", root), "utf8"),
  readFile(new URL("desktop/src/ui.rs", root), "utf8"),
  readFile(new URL("desktop/backend/src/service-contract.json", root), "utf8"),
])

test("게임 오버레이 설정창은 다섯 알림과 디버프 자동인식을 제공한다", () => {
  for (const label of ["디버프 알림", "투아림 알림", "붕괴 알림", "버프 알림", "스킬 알림"]) {
    assert.match(settings, new RegExp(label))
  }
  for (const key of [
    "debuffNotifications",
    "tuarimNotifications",
    "collapseNotifications",
    "buffNotifications",
    "skillNotifications",
  ]) {
    assert.match(settings, new RegExp(key))
  }
  assert.match(settings, /인터페이스 자동인식/)
  assert.match(settings, /인터페이스 구성/)
  assert.match(settings, /detect\.disabled = busy/)
  assert.doesNotMatch(settings, /detect\.disabled = busy \|\|/)
  assert.match(settings, /configure\.disabled = busy/)
  assert.doesNotMatch(settings, /configure\.disabled = busy \|\|/)
  assert.match(settings, /body \{[^}]*overflow: hidden/)
  assert.match(settingsPreload, /overlay-settings:get-status/)
  assert.match(settingsPreload, /overlay-settings:set-setting/)
  assert.match(settingsPreload, /overlay-settings:detect-debuff-region/)
  assert.match(settingsPreload, /overlay-settings:configure-debuff-alert/)
  assert.match(ui, /게임 오버레이[\s\S]+application:open-overlay-settings/)
})

test("디버프 누락 알림은 타입별 레이어와 이름을 표시한다", () => {
  assert.match(alert, /디버프 누락/)
  assert.match(alert, /debuffAlertConfiguring/)
  for (const layer of ["물리", "마법", "보너스", "아르카나", "이동속도"]) {
    assert.match(alert, new RegExp(`"${layer}"`))
  }
  assert.match(alert, /\["물리", \[1, 3\]\]/)
  assert.match(alert, /\["마법", \[2, 3\]\]/)
  assert.match(alert, /name\.textContent = debuff\.name/)
  assert.match(alertPreload, /debuff-alert:get-status/)
  assert.match(alertPreload, /debuff-alert:drag-start/)
  assert.match(alertPreload, /ipcRenderer\.send\("debuff-alert:drag-start"/)
  assert.doesNotMatch(alertPreload, /finish/)
  assert.match(windows, /"debuff-alert-overlay"[\s\S]+transparent/)
  assert.match(windows, /"debuff-alert-overlay"[\s\S]+?"width":520,"height":440/)
  assert.match(windows, /show_debuff_alert/)
  assert.match(windows, /self\.command\(id, "dragWindow"/)
  assert.match(native, /"dragWindow" => window\.drag_window\(\)/)
  assert.match(service, /"debuff-alert" => kind == "debuff-alert-overlay"/)
  assert.match(service, /monitor_debuff_alert/)
})

test("인터페이스 구성 완료는 독립 제어 창에서 처리한다", () => {
  assert.match(configuration, /인터페이스 구성/)
  assert.match(configuration, /구성 완료/)
  assert.match(configurationPreload, /overlay-configuration:drag-start/)
  assert.match(configurationPreload, /overlay-configuration:finish/)
  assert.match(windows, /"overlay-configuration"[\s\S]+?"width":300,"height":112/)
  assert.match(service, /destroy_debuff_preview/)
  assert.match(service, /destroy_overlay_configuration/)
})

test("디버프 영역 preview는 구성 중에만 체력바 위 영역을 표시한다", () => {
  assert.match(preview, /디버프 영역/)
  assert.match(preview, /pointer-events:\s*none/)
  assert.match(previewPreload, /use strict/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+transparent/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+alwaysOnTop/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+focusable":false/)
  assert.match(windows, /show_debuff_preview/)
  assert.match(service, /monitor_debuff_preview/)
  const previewMonitor = service.match(/fn monitor_debuff_preview[\s\S]*?(?=fn monitor_debuff_alert)/)?.[0] ?? ""
  assert.doesNotMatch(previewMonitor, /debuff_preview_gate/)
  assert.match(windows, /"overlay-settings"[\s\S]+?"height":700/)
  assert.doesNotMatch(windows, /matches!\(w\.kind\.as_str\(\), "blackbox-editor" \| "overlay-settings"\)/)
})

test("오버레이 문서·IPC·package 입력은 native 계약에 등록된다", () => {
  assert.match(service, /"overlay-settings"/)
  assert.match(service, /overlay-settings:/)
  assert.match(native, /"overlay-settings\.html"/)
  assert.match(native, /"debuff-region-overlay\.html"/)
  assert.match(native, /"debuff-alert-overlay\.html"/)
  assert.match(native, /"overlay-configuration\.html"/)
  assert.match(packager, /overlay-settings\.html/)
  assert.match(packager, /debuff-region-overlay\.html/)
  assert.match(packager, /debuff-alert-overlay\.html/)
  assert.match(packager, /overlay-configuration\.html/)
  assert.match(packager, /data\/conditions/)
  for (const channel of [
    "application:open-overlay-settings",
    "overlay-settings:request-close",
    "overlay-settings:get-status",
    "overlay-settings:set-setting",
    "overlay-settings:detect-debuff-region",
    "overlay-settings:configure-debuff-alert",
    "debuff-alert:get-status",
    "debuff-alert:drag-start",
    "overlay-configuration:drag-start",
    "overlay-configuration:finish",
  ]) {
    assert.match(contract, new RegExp(channel))
  }
})
