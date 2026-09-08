import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  blackboxFeatureAvailable,
  bitrateForBlackboxSetting,
  defaultBlackboxSetting,
  maxHeightForBlackboxQuality,
  normalizeBlackboxSetting,
  resolveAutoBlackboxQuality,
} from "../src/blackbox-settings.mjs"

const root = new URL("../", import.meta.url)

test("블랙박스 설정은 안전한 기본값과 허용된 선택지만 사용한다", () => {
  assert.equal(blackboxFeatureAvailable, true)
  assert.deepEqual(normalizeBlackboxSetting(null), defaultBlackboxSetting)
  assert.deepEqual(normalizeBlackboxSetting({
    enabled: true,
    codec: "av1",
    capacityGb: 999,
    clipSeconds: 17,
    fps: 144,
  }), {
    ...defaultBlackboxSetting,
    featureEnabled: true,
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
    featureEnabled: true,
    enabled: true,
    codec: "hevc",
    quality: "auto",
    capacityGb: 100,
    clipSeconds: 60,
    fps: 30,
    chunkSeconds: 4,
  })
  assert.equal(normalizeBlackboxSetting({
    featureEnabled: true,
    enabled: false,
  }).featureEnabled, true)
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

test("메인 버튼과 전용 관리 창에 블랙박스 제어가 연결된다", async () => {
  const [
    appSource,
    styleSource,
    mainSource,
    preloadSource,
    managerPreloadSource,
    managerSource,
    packageSource,
  ] = await Promise.all([
    readFile(new URL("web/App.svelte", root), "utf8"),
    readFile(new URL("web/styles.css", root), "utf8"),
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/preload.cjs", root), "utf8"),
    readFile(new URL("electron/blackbox-manager-preload.cjs", root), "utf8"),
    readFile(new URL("blackbox-manager.html", root), "utf8"),
    readFile(new URL("package.json", root), "utf8"),
  ])
  assert.match(appSource, /class="blackbox-main-link"/)
  assert.match(appSource, /class="blackbox-window-link"/)
  assert.match(appSource, /\{#if blackboxFeatureAvailable && blackboxFeatureEnabled\}[\s\S]*class="blackbox-main-controls"/)
  assert.match(
    appSource,
    /<h2>게임 블랙박스<\/h2>[\s\S]*onclick=\{toggleBlackboxFeature\}[\s\S]*blackboxFeatureEnabled \? "사용 중" : "사용하기"/,
  )
  assert.match(appSource, /onclick=\{toggleMainBlackbox\}/)
  assert.match(
    appSource,
    /async function syncBlackboxSetting\(\) \{[\s\S]*if \(!blackboxSettingLoaded \|\| blackboxTogglePending\) return/,
  )
  assert.match(styleSource, /\.blackbox-main-link:disabled \{[\s\S]*opacity: 1/)
  assert.match(
    appSource,
    /function holdBlackboxTransitionMask\(\)[\s\S]*blackboxTransitionPhase = "hold"[\s\S]*if \(!blackboxTogglePending\) revealBlackboxTransitionTarget\(\)/,
  )
  assert.match(appSource, /blackboxTransitionPhase === "hold"[\s\S]*class="blackbox-text-over holding"/)
  assert.match(styleSource, /\.blackbox-text-over \{[\s\S]*color: transparent/)
  assert.match(styleSource, /\.blackbox-text-over\.holding \{[\s\S]*animation: none/)
  assert.match(appSource, /blackboxTransitionPhase === "enter"[\s\S]*class="blackbox-icon-mask"/)
  assert.match(appSource, /blackboxTransitionPhase === "hold"[\s\S]*class="blackbox-icon-mask holding"/)
  assert.match(appSource, /blackboxTransitionPhase === "leave"[\s\S]*class="blackbox-icon-mask leaving"/)
  assert.match(styleSource, /\.blackbox-icon-mask \{[\s\S]*animation: chicken-text-in 80ms 270ms linear both/)
  assert.match(styleSource, /\.blackbox-icon-mask\.leaving \{[\s\S]*animation: chicken-text-out 80ms 270ms linear both/)
  assert.doesNotMatch(styleSource, /blackbox-pending-mask/)
  assert.match(
    appSource,
    /function revealBlackboxTransitionTarget\(\)[\s\S]*blackboxDisplayedText = blackboxTransitionTo[\s\S]*blackboxDisplayedEnabled = blackboxTransitionToEnabled[\s\S]*blackboxTransitionPhase = "leave"/,
  )
  assert.match(appSource, /openBlackboxManager/)
  assert.match(
    mainSource,
    /application:set-blackbox-feature-enabled[\s\S]*featureEnabled: nextFeatureEnabled,[\s\S]*enabled: nextFeatureEnabled && current\.enabled/,
  )
  assert.match(mainSource, /application:set-blackbox-enabled[\s\S]*\.\.\.current,[\s\S]*enabled: Boolean\(enabled\)/)
  assert.match(
    mainSource,
    /async function ensureBlackboxStarted\(\)[\s\S]*if \(!blackboxFeatureAvailable\)[\s\S]*featureEnabled: false,[\s\S]*enabled: false/,
  )
  assert.match(preloadSource, /setBlackboxEnabled/)
  assert.match(preloadSource, /setBlackboxFeatureEnabled/)
  assert.match(mainSource, /function openBlackboxManager\(\)/)
  assert.match(mainSource, /title: "게임 블랙박스 관리"/)
  assert.match(mainSource, /blackbox-manager\.html/)
  assert.match(managerSource, /게임 블랙박스 관리/)
  assert.match(managerSource, /Ctrl\+Shift\+F10/)
  assert.match(managerSource, /data-page="extract">영상 추출/)
  assert.match(managerSource, /data-page="clips">저장된 클립/)
  assert.match(managerSource, /data-page="settings">녹화 설정/)
  assert.match(managerSource, /blackbox-editor\.html\?embedded=1/)
  assert.match(managerSource, /class="recording-badge"[\s\S]*class="recording-dot"/)
  assert.doesNotMatch(managerSource, /class="heading-usage"/)
  assert.match(managerSource, /class="clip-browser"[\s\S]*class="clip-preview"/)
  assert.doesNotMatch(managerSource, /class="status-grid"/)
  assert.doesNotMatch(managerSource, /class="audio-state"/)
  assert.match(managerSource, /전체 비우기/)
  assert.match(managerPreloadSource, /blackbox-manager:set-setting/)
  assert.match(managerPreloadSource, /blackbox-manager:clear-recording/)
  assert.match(managerPreloadSource, /blackbox-manager:list-clips/)
  assert.match(managerPreloadSource, /blackbox-manager:open-clip/)
  assert.match(managerPreloadSource, /blackbox-manager:fit-media/)
  assert.match(managerPreloadSource, /blackbox-manager:rename-clip/)
  assert.match(managerPreloadSource, /blackbox-manager:delete-clip/)
  assert.match(mainSource, /async function listBlackboxClips\(\)/)
  assert.match(mainSource, /function fitBlackboxManagerToMedia\(value\)/)
  assert.match(mainSource, /screen\.getDisplayMatching\(bounds\)\.workArea/)
  assert.match(mainSource, /async function renameBlackboxClip\(fileName, requestedName\)/)
  assert.match(mainSource, /async function deleteBlackboxClip\(fileName\)/)
  assert.match(mainSource, /같은 이름의 클립이 이미 있습니다/)
  assert.match(mainSource, /windowStatePath: join\(directory, "window\.json"\)/)
  assert.match(mainSource, /function readBlackboxManagerSize\(\)/)
  assert.match(mainSource, /function openBlackboxManager\(\)[\s\S]*window\.on\("will-resize"/)
  assert.match(mainSource, /function openBlackboxManager\(\)[\s\S]*window\.on\("resize"/)
  assert.match(mainSource, /blackboxManagerAutomaticBounds/)
  assert.match(mainSource, /blackboxManagerPreferredSize/)
  assert.match(mainSource, /const desiredViewportHeight = viewportWidth \/ ratio/)
  assert.match(mainSource, /const desiredViewportWidth = viewportHeight \* ratio/)
  assert.match(mainSource, /bounds\.x - \(targetWidth - bounds\.width\) \/ 2/)
  assert.match(managerSource, /grid-template-columns: 290px minmax\(0, 1fr\)/)
  assert.match(managerSource, /\.clip-browser \{[\s\S]*width: 290px;[\s\S]*min-width: 290px;[\s\S]*max-width: 290px;/)
  assert.match(managerSource, /function beginClipRename\(clip, item, nameButton\)/)
  assert.match(managerSource, /function removeClip\(clip\)/)
  assert.match(managerSource, /!selectedClipName && clip === clips\[0\]/)
  assert.match(managerSource, /clipVideo\.addEventListener\("loadedmetadata", fitSavedClipMedia\)/)
  assert.match(managerSource, /source: "blackbox-manager-status"/)
  const managerScript = managerSource.match(/<script>([\s\S]*)<\/script>/)?.[1]
  assert.ok(managerScript)
  assert.doesNotThrow(() => new Function(managerScript))
  assert.match(mainSource, /return clips\.sort\(\(left, right\) => right\.modifiedAt - left\.modifiedAt\)/)
  assert.match(mainSource, /nogirem-blackbox:\/\/clips\//)
  assert.match(mainSource, /application:get-blackbox-setting/)
  assert.match(mainSource, /application:save-blackbox-clip/)
  assert.match(mainSource, /application:clear-blackbox-recording/)
  assert.match(mainSource, /rm\(join\(paths\.storagePath, "Ring"\)/)
  assert.match(mainSource, /저장된 클립은 삭제하지 않습니다/)
  assert.match(preloadSource, /getBlackboxSetting/)
  assert.match(preloadSource, /clearBlackboxRecording/)
  assert.match(packageSource, /native\/recorder-helper\/bin\/recorder-helper\.exe/)
  assert.match(packageSource, /blackbox-manager\.html/)
})

test("고정 시점 블랙박스 추출 편집 창과 구간 remux가 연결된다", async () => {
  const [
    managerSource,
    mainSource,
    preloadSource,
    editorSource,
    editorScript,
    nativeSource,
    viteSource,
  ] = await Promise.all([
    readFile(new URL("blackbox-manager.html", root), "utf8"),
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/blackbox-editor-preload.cjs", root), "utf8"),
    readFile(new URL("blackbox-editor.html", root), "utf8"),
    readFile(new URL("web/blackbox-editor.js", root), "utf8"),
    readFile(new URL("native/recorder-helper/main.cpp", root), "utf8"),
    readFile(new URL("vite.config.mjs", root), "utf8"),
  ])
  assert.match(managerSource, /영상 추출/)
  assert.match(managerSource, /class="extract-frame"/)
  assert.match(managerSource, /window\.blackboxManager\.editor/)
  assert.match(mainSource, /blackbox-manager:get-editor-session/)
  assert.match(mainSource, /blackbox-manager:set-track-seconds/)
  assert.match(mainSource, /blackbox-manager:extract/)
  assert.match(mainSource, /title: "블랙박스 영상 추출"[\s\S]*alwaysOnTop: true/)
  assert.match(mainSource, /closeWindowOnEscape\(window\)/)
  assert.match(mainSource, /async function latestCompletedBlackboxAnchor/)
  assert.match(mainSource, /\^chunk-\(\\d\+\)\\\.mp4\$/)
  assert.doesNotMatch(mainSource, /flushBlackboxForEditor/)
  assert.match(mainSource, /protocol\.handle\("nogirem-blackbox"/)
  assert.match(mainSource, /async function localVideoResponse/)
  assert.match(mainSource, /"Accept-Ranges": "bytes"/)
  assert.match(mainSource, /status = 206/)
  assert.match(mainSource, /"Content-Range"/)
  assert.match(mainSource, /queueBlackboxControlOperation/)
  assert.match(mainSource, /videoUrl: `nogirem-blackbox:\/\/editor\//)
  assert.match(preloadSource, /blackbox-editor:set-track-seconds/)
  assert.match(preloadSource, /blackbox-editor:extract/)
  assert.match(editorSource, /class="compact-button add-time"[^>]*>\+</)
  assert.match(editorSource, /class="compact-button direct-time"[^>]*>\+\+</)
  assert.match(editorSource, /블랙박스 조회 길이/)
  assert.match(editorSource, /id="track-minutes"[\s\S]*>분<\/span>/)
  assert.match(editorSource, /id="track-seconds"[\s\S]*>초<\/span>/)
  assert.match(editorSource, /class="selection-guide"/)
  assert.match(editorSource, /class="selection-handle start"/)
  assert.match(editorSource, /class="selection-handle end"/)
  assert.match(editorSource, /class="track-gaps"/)
  assert.match(editorSource, /class="track-status"/)
  assert.match(editorScript, /requestedTrackSeconds \+ 30/)
  assert.match(editorScript, /function normalizeTrackLengthInputs\(\)/)
  assert.match(editorScript, /function fitCurrentMedia\(\)/)
  assert.match(editorScript, /page: "extract"/)
  assert.match(managerSource, /page: "clips"/)
  assert.match(editorScript, /function renderManagerStatus\(value\)/)
  assert.match(editorScript, /seconds >= 60 \? seconds : minutes \* 60 \+ seconds/)
  assert.match(editorScript, /minutes \* 60 \+ seconds/)
  assert.match(editorScript, /Math\.floor\(normalized \/ 60\)/)
  assert.match(editorScript, /normalized % 60/)
  assert.match(editorScript, /requestParent\("getSession"\)/)
  assert.match(editorScript, /let requestedTrackSeconds = 60/)
  assert.match(editorScript, /event\.code !== "Space"/)
  assert.match(editorScript, /function beginTimelineInteraction\(event\)/)
  assert.match(editorScript, /mode = "resize-start"/)
  assert.match(editorScript, /mode = "resize-end"/)
  assert.match(editorScript, /if \(mode === "seek" \|\| mode === "pending-move"\) seekFromPointer\(event\)/)
  assert.match(editorScript, /video\.pause\(\)\s*video\.currentTime = selectionStart\s*previewingSelection = false/)
  assert.match(editorScript, /targetTime >= selectionStart/)
  assert.match(editorScript, /video\.currentTime < selectionEnd - 0\.01/)
  assert.match(editorScript, /function renderTrackGaps\(\)/)
  assert.match(editorScript, /element\.className = "track-gap"/)
  assert.match(mainSource, /return createBlackboxEditorTrack\(session, 60\)/)
  assert.match(mainSource, /const metadataOutput = await runRecorderUtility/)
  assert.match(mainSource, /Array\.isArray\(parsed\?\.gaps\)/)
  assert.match(editorScript, /previewSelection/)
  assert.match(nativeSource, /mode == L"track"/)
  assert.match(nativeSource, /mode == L"extract"/)
  assert.match(nativeSource, /runUtilityMode[\s\S]*NORMAL_PRIORITY_CLASS/)
  assert.match(nativeSource, /status\.fps = options\.fps;\s*SetPriorityClass\(GetCurrentProcess\(\), BELOW_NORMAL_PRIORITY_CLASS\)/)
  assert.match(nativeSource, /MF_READWRITE_DISABLE_CONVERTERS/)
  assert.match(nativeSource, /MFSampleExtension_DecodeTimestamp/)
  assert.match(nativeSource, /AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK/)
  assert.match(nativeSource, /PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE/)
  assert.match(nativeSource, /steadyTimeFromQpc100Nanoseconds/)
  assert.match(nativeSource, /AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR/)
  assert.match(nativeSource, /AvSetMmThreadCharacteristicsW/)
  assert.match(nativeSource, /MaximumQueuedAudioFrames/)
  assert.match(nativeSource, /class AudioGainController/)
  assert.match(nativeSource, /AudioMaximumGain/)
  assert.match(nativeSource, /AudioNoiseFloor/)
  assert.match(nativeSource, /AudioLimiterPeak/)
  assert.match(nativeSource, /AudioGainReleaseSeconds/)
  assert.match(nativeSource, /MFAudioFormat_AAC/)
  assert.match(nativeSource, /MF_SOURCE_READER_FIRST_AUDIO_STREAM/)
  assert.match(nativeSource, /audioRecording/)
  assert.match(nativeSource, /compatibleChunkSuffix/)
  assert.match(nativeSource, /MF_PD_DURATION/)
  assert.match(nativeSource, /LONGLONG compressedMediaDuration/)
  assert.match(nativeSource, /duration \+= compressedMediaDuration\(input\)/)
  assert.match(nativeSource, /createPcmAudioReader/)
  assert.match(nativeSource, /createAacAudioType/)
  assert.match(nativeSource, /combinedDecodedAudioDuration/)
  assert.match(nativeSource, /sample->DeleteItem\(MFSampleExtension_DecodeTimestamp\)/)
  assert.doesNotMatch(nativeSource, /trackGapsJson/)
  assert.match(nativeSource, /std::cout << "\{\\"gaps\\":\[\]\}\\n"/)
  assert.match(nativeSource, /removeIncompleteChunks/)
  assert.match(nativeSource, /clearRingDirectory/)
  assert.match(nativeSource, /clearCompletedId/)
  assert.match(nativeSource, /discardQueuedVideoFrames/)
  assert.match(nativeSource, /writerPublisherThread_/)
  assert.match(nativeSource, /closeWriter\(bool waitForPublish = false\)/)
  assert.match(nativeSource, /durationSeconds/)
  assert.match(nativeSource, /name\.find\(L"\.partial\."\)/)
  assert.match(nativeSource, /combinedMediaDuration/)
  assert.match(nativeSource, /totalDuration - requestedDuration/)
  assert.match(nativeSource, /\.partial\.mp4/)
  assert.match(viteSource, /blackboxEditor: resolve\("blackbox-editor\.html"\)/)
})
