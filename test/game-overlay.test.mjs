import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"

const root = new URL("../", import.meta.url)
const [
  settings,
  preview,
  settingsPreload,
  previewPreload,
  service,
  windows,
  native,
  packager,
  ui,
  contract,
] = await Promise.all([
  readFile(new URL("overlay-settings.html", root), "utf8"),
  readFile(new URL("debuff-region-overlay.html", root), "utf8"),
  readFile(new URL("service/overlay-settings-preload.js", root), "utf8"),
  readFile(new URL("service/debuff-region-overlay-preload.js", root), "utf8"),
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
  assert.match(settingsPreload, /overlay-settings:get-status/)
  assert.match(settingsPreload, /overlay-settings:set-setting/)
  assert.match(settingsPreload, /overlay-settings:detect-debuff-region/)
  assert.match(ui, /게임 오버레이[\s\S]+application:open-overlay-settings/)
})

test("디버프 영역 preview는 독립된 클릭 통과 native 창이다", () => {
  assert.match(preview, /디버프 영역/)
  assert.match(preview, /pointer-events:\s*none/)
  assert.match(previewPreload, /use strict/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+transparent/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+alwaysOnTop/)
  assert.match(windows, /"debuff-region-overlay"[\s\S]+focusable":false/)
  assert.match(windows, /show_debuff_preview/)
  assert.match(service, /monitor_debuff_preview/)
})

test("오버레이 문서·IPC·package 입력은 native 계약에 등록된다", () => {
  assert.match(service, /"overlay-settings"/)
  assert.match(service, /overlay-settings:/)
  assert.match(native, /"overlay-settings\.html"/)
  assert.match(native, /"debuff-region-overlay\.html"/)
  assert.match(packager, /overlay-settings\.html/)
  assert.match(packager, /debuff-region-overlay\.html/)
  for (const channel of [
    "application:open-overlay-settings",
    "overlay-settings:request-close",
    "overlay-settings:get-status",
    "overlay-settings:set-setting",
    "overlay-settings:detect-debuff-region",
  ]) {
    assert.match(contract, new RegExp(channel))
  }
})
