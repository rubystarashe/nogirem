import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  bitrateForBlackboxSetting,
  defaultBlackboxSetting,
  maxHeightForBlackboxQuality,
  normalizeBlackboxSetting,
  resolveAutoBlackboxQuality,
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
    quality: "auto",
    capacityGb: 100,
    clipSeconds: 60,
    fps: 30,
  }), {
    enabled: true,
    codec: "hevc",
    quality: "auto",
    capacityGb: 100,
    clipSeconds: 60,
    fps: 30,
    chunkSeconds: 4,
  })
})

test("H.264와 HEVC 프리셋은 프레임별 비트레이트를 제공한다", () => {
  assert.equal(bitrateForBlackboxSetting({ codec: "h264", quality: "1080p", fps: 60 }), 12)
  assert.equal(bitrateForBlackboxSetting({ codec: "h264", quality: "1440p", fps: 60 }), 24)
  assert.equal(bitrateForBlackboxSetting({ codec: "hevc", quality: "1440p", fps: 60 }), 16)
  assert.equal(bitrateForBlackboxSetting({ codec: "hevc", quality: "original", fps: 30 }), 14)
})

test("자동 화질은 CPU와 메모리에 따라 1080p 또는 1440p를 선택한다", () => {
  assert.equal(resolveAutoBlackboxQuality({
    logicalCpuCount: 12,
    totalMemoryBytes: 16 * 1024 ** 3,
  }), "1440p")
  assert.equal(resolveAutoBlackboxQuality({
    logicalCpuCount: 8,
    totalMemoryBytes: 32 * 1024 ** 3,
  }), "1080p")
  assert.equal(maxHeightForBlackboxQuality("1080p"), 1080)
  assert.equal(maxHeightForBlackboxQuality("1440p"), 1440)
  assert.equal(maxHeightForBlackboxQuality("original"), 0)
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

test("고정 시점 블랙박스 추출 편집 창과 구간 remux가 연결된다", async () => {
  const [
    appSource,
    mainSource,
    preloadSource,
    editorSource,
    editorScript,
    nativeSource,
    viteSource,
  ] = await Promise.all([
    readFile(new URL("web/App.svelte", root), "utf8"),
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/blackbox-editor-preload.cjs", root), "utf8"),
    readFile(new URL("blackbox-editor.html", root), "utf8"),
    readFile(new URL("web/blackbox-editor.js", root), "utf8"),
    readFile(new URL("native/recorder-helper/main.cpp", root), "utf8"),
    readFile(new URL("vite.config.mjs", root), "utf8"),
  ])
  assert.match(appSource, /영상 추출/)
  assert.match(mainSource, /title: "블랙박스 영상 추출"[\s\S]*alwaysOnTop: true/)
  assert.match(mainSource, /closeWindowOnEscape\(window\)/)
  assert.match(mainSource, /command: "flush"/)
  assert.match(mainSource, /protocol\.handle\("nogirem-blackbox"/)
  assert.match(mainSource, /queueBlackboxControlOperation/)
  assert.match(mainSource, /videoUrl: `nogirem-blackbox:\/\/editor\//)
  assert.match(preloadSource, /blackbox-editor:set-track-seconds/)
  assert.match(preloadSource, /blackbox-editor:extract/)
  assert.match(editorSource, /class="compact-button add-time"[^>]*>\+</)
  assert.match(editorSource, /class="compact-button direct-time"[^>]*>\+\+</)
  assert.match(editorSource, /class="selection-guide"/)
  assert.match(editorScript, /requestedTrackSeconds \+ 30/)
  assert.match(editorScript, /previewSelection/)
  assert.match(nativeSource, /mode == L"track"/)
  assert.match(nativeSource, /mode == L"extract"/)
  assert.match(nativeSource, /MF_READWRITE_DISABLE_CONVERTERS/)
  assert.match(nativeSource, /MFSampleExtension_DecodeTimestamp/)
  assert.match(nativeSource, /AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK/)
  assert.match(nativeSource, /PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE/)
  assert.match(nativeSource, /MFAudioFormat_AAC/)
  assert.match(nativeSource, /MF_SOURCE_READER_FIRST_AUDIO_STREAM/)
  assert.match(nativeSource, /audioRecording/)
  assert.match(nativeSource, /compatibleChunkSuffix/)
  assert.match(nativeSource, /combinedMediaDuration/)
  assert.match(nativeSource, /totalDuration - requestedDuration/)
  assert.match(nativeSource, /\.partial\.mp4/)
  assert.match(viteSource, /blackboxEditor: resolve\("blackbox-editor\.html"\)/)
})
