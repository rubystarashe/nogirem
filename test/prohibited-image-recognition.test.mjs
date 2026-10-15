import assert from "node:assert/strict"
import { access, readFile } from "node:fs/promises"
import test from "node:test"

const root = new URL("../", import.meta.url)
const removedPaths = [
  "desktop/backend/src/game_overlay.rs",
  "overlay-settings.html",
  "debuff-region-overlay.html",
  "debuff-alert-overlay.html",
  "overlay-configuration.html",
  "debuff.csv",
  "service/overlay-settings-preload.js",
  "service/debuff-region-overlay-preload.js",
  "service/debuff-alert-overlay-preload.js",
  "service/overlay-configuration-preload.js",
]

test("게임 화면 이미지 인식 구현과 제품 진입점은 존재하지 않는다", async () => {
  for (const path of removedPaths) {
    await assert.rejects(access(new URL(path, root)), `${path}가 다시 추가되었습니다`)
  }

  const currentSources = await Promise.all([
    "desktop/backend/src/lib.rs",
    "desktop/backend/src/service.rs",
    "desktop/backend/src/service_windows.rs",
    "desktop/backend/src/service-contract.json",
    "desktop/src/native.rs",
    "desktop/src/ui.rs",
    "scripts/package-dioxus.mjs",
    "service/main.mjs",
  ].map(path => readFile(new URL(path, root), "utf8")))

  const prohibited = /game_overlay|overlay-settings|debuff-alert|debuff-region-overlay|overlay-configuration|debuff\.csv|data\/conditions/
  for (const source of currentSources) assert.doesNotMatch(source, prohibited)
})
