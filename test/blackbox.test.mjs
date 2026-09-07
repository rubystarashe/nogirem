import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  bitrateForBlackboxSetting,
  defaultBlackboxSetting,
  normalizeBlackboxSetting,
} from "../src/blackbox-settings.mjs"

const root = new URL("../", import.meta.url)

test("블랙박스 설정은 안전한 기본값과 허용된 선택지만 사용한다", () => {
  assert.deepEqual(normalizeBlackboxSetting(null), defaultBlackboxSetting)
  assert.deepEqual(normalizeBlackboxSetting({
    enabled: true,
    codec: "av1",
    capacityGb: 999,
    clipSeconds: 17,
    fps: 144,
  }), {
    ...defaultBlackboxSetting,
    enabled: true,
  })
  assert.deepEqual(normalizeBlackboxSetting({
    enabled: true,
    codec: "hevc",
    capacityGb: 100,
    clipSeconds: 60,
    fps: 30,
  }), {
    enabled: true,
    codec: "hevc",
    capacityGb: 100,
    clipSeconds: 60,
    fps: 30,
    chunkSeconds: 4,
  })
})

test("H.264와 HEVC 프리셋은 프레임별 비트레이트를 제공한다", () => {
  assert.equal(bitrateForBlackboxSetting({ codec: "h264", fps: 60 }), 12)
  assert.equal(bitrateForBlackboxSetting({ codec: "h264", fps: 30 }), 7)
  assert.equal(bitrateForBlackboxSetting({ codec: "hevc", fps: 60 }), 8)
  assert.equal(bitrateForBlackboxSetting({ codec: "hevc", fps: 30 }), 5)
})

test("고급 기능과 Electron IPC에 블랙박스 제어가 연결된다", async () => {
  const [appSource, mainSource, preloadSource, packageSource] = await Promise.all([
    readFile(new URL("web/App.svelte", root), "utf8"),
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/preload.cjs", root), "utf8"),
    readFile(new URL("package.json", root), "utf8"),
  ])
  assert.match(appSource, /<h2>게임 블랙박스<\/h2>/)
  assert.match(appSource, /Ctrl\+Shift\+F10/)
  assert.match(mainSource, /application:get-blackbox-setting/)
  assert.match(mainSource, /application:save-blackbox-clip/)
  assert.match(preloadSource, /getBlackboxSetting/)
  assert.match(packageSource, /native\/recorder-helper\/bin\/recorder-helper\.exe/)
})
