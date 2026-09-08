import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  blackboxFeatureAvailable,
  bitrateForBlackboxSetting,
  defaultBlackboxShortcut,
  defaultBlackboxSetting,
  maxHeightForBlackboxQuality,
  normalizeBlackboxShortcut,
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
    maxDurationSeconds: 3600,
    clipSeconds: 60,
    fps: 30,
  }), {
    featureEnabled: true,
    enabled: true,
    codec: "hevc",
    quality: "auto",
    capacityGb: 100,
    maxDurationSeconds: 3600,
    clipSeconds: 60,
    fps: 30,
    chunkSeconds: 10,
    shortcut: defaultBlackboxShortcut,
  })
  assert.equal(normalizeBlackboxSetting({
    featureEnabled: true,
    enabled: false,
  }).featureEnabled, true)
  assert.equal(normalizeBlackboxSetting({
    maxDurationSeconds: 100,
  }).maxDurationSeconds, 100)
  assert.equal(normalizeBlackboxSetting({
    maxDurationSeconds: 30,
  }).maxDurationSeconds, 60)
})

test("빠른 클립 저장 단축키는 지원하는 키 조합만 정규화한다", () => {
  assert.equal(normalizeBlackboxShortcut("ctrl+alt+k"), "Control+Alt+K")
  assert.equal(normalizeBlackboxShortcut("CommandOrControl+Shift+F12"), "CommandOrControl+Shift+F12")
  assert.equal(normalizeBlackboxShortcut("PauseBreak"), "Pause")
  assert.equal(normalizeBlackboxShortcut("Ctrl+Pause"), "Pause")
  assert.equal(normalizeBlackboxShortcut(""), "")
  assert.equal(normalizeBlackboxShortcut("F10"), defaultBlackboxShortcut)
  assert.equal(normalizeBlackboxShortcut("Ctrl+한"), defaultBlackboxShortcut)
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
  assert.match(appSource, /class="blackbox-main-duration"[\s\S]*class:active=\{blackboxDisplayedEnabled\}[\s\S]*\{blackboxDurationText\(\)\}/)
  assert.match(appSource, /function blackboxDurationText\(\)/)
  assert.doesNotMatch(appSource, /`블박 \$\{/)
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
  assert.match(styleSource, /\.blackbox-main-duration \{[\s\S]*font-size: 9px[\s\S]*transform: translateX\(28px\)/)
  assert.match(styleSource, /\.blackbox-main-duration\.active \{[\s\S]*0\.72[\s\S]*font-weight: 650/)
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
    /application:set-blackbox-feature-enabled[\s\S]*featureEnabled: nextFeatureEnabled,[\s\S]*enabled: nextFeatureEnabled/,
  )
  assert.match(
    appSource,
    /async function toggleBlackboxFeature\(\)[\s\S]*applyBlackboxState\(\{ featureEnabled: true, enabled: true \}\)[\s\S]*closeCreatorView\(\)/,
  )
  assert.match(
    mainSource,
    /async function setBlackboxEnabled\(enabled\)[\s\S]*\.\.\.current,[\s\S]*enabled: Boolean\(enabled\)/,
  )
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
  assert.match(managerSource, /Ctrl \+ Shift \+ F10/)
  assert.match(managerSource, /data-page="extract">영상 추출/)
  assert.match(managerSource, /data-page="clips">저장된 클립/)
  assert.match(managerSource, /data-page="settings">녹화 설정/)
  assert.match(managerSource, /blackbox-editor\.html\?embedded=1/)
  assert.match(managerSource, /class="recording-badge"[\s\S]*class="recording-dot"/)
  assert.doesNotMatch(managerSource, /class="heading-usage"/)
  assert.match(managerSource, /class="clip-browser"[\s\S]*class="clip-preview"/)
  assert.doesNotMatch(managerSource, /class="status-grid"/)
  assert.doesNotMatch(managerSource, /class="audio-state"/)
  assert.doesNotMatch(managerSource, />전체 비우기</)
  assert.match(managerSource, /class="clip-save-modal" hidden/)
  assert.match(managerSource, /class="clip-delete-modal" hidden/)
  assert.match(managerSource, /class="action save-clip clip-save-header"/)
  assert.match(managerSource, /return `\$\{highest \+ 1\}번째 클립`/)
  assert.match(managerSource, /clipSaveNameInput\.value\.trim\(\)[\s\S]*clipSaveModal\.dataset\.fallbackName/)
  assert.match(managerSource, /window\.blackboxManager\.saveClip\(clipName\)/)
  assert.match(managerSource, /\.settings \{[\s\S]*display: block/)
  assert.match(managerSource, /\.settings label \{[\s\S]*display: flex[\s\S]*border-bottom/)
  assert.doesNotMatch(managerSource, /class="action primary toggle-recording"/)
  assert.match(managerSource, /\.settings-page \.page-actions \{[\s\S]*justify-content: flex-end/)
  assert.match(managerSource, /class="message settings-message"[\s\S]*class="action save-setting/)
  assert.match(managerSource, /function openClipDeleteModal\(clip\)/)
  assert.match(managerSource, /const scrollTop = clipList\.scrollTop[\s\S]*clipList\.scrollTop = scrollTop/)
  assert.doesNotMatch(managerSource, /<footer class="actions">/)
  assert.match(managerPreloadSource, /blackbox-manager:set-setting/)
  assert.match(managerPreloadSource, /saveClip: requestedName/)
  assert.match(managerPreloadSource, /onSaveClipRequested/)
  assert.match(managerPreloadSource, /blackbox-manager:set-enabled/)
  assert.match(managerPreloadSource, /onEditorExtractProgress/)
  assert.match(managerPreloadSource, /blackbox-manager:clear-recording/)
  assert.match(managerPreloadSource, /blackbox-manager:list-clips/)
  assert.match(managerPreloadSource, /blackbox-manager:open-clip/)
  assert.match(managerPreloadSource, /blackbox-manager:fit-media/)
  assert.match(managerPreloadSource, /blackbox-manager:rename-clip/)
  assert.match(managerPreloadSource, /blackbox-manager:delete-clip/)
  assert.match(managerPreloadSource, /blackbox-manager:escape-pressed/)
  assert.match(mainSource, /async function listBlackboxClips\(\)/)
  assert.match(mainSource, /function fitBlackboxManagerToMedia\(value\)/)
  assert.match(mainSource, /screen\.getDisplayMatching\(bounds\)\.workArea/)
  assert.match(mainSource, /async function renameBlackboxClip\(fileName, requestedName\)/)
  assert.match(mainSource, /async function openBlackboxClip\(fileName\)[\s\S]*shell\.showItemInFolder\(clipPath\)/)
  assert.match(mainSource, /async function requestBlackboxClip\(requestedName = ""\)/)
  assert.match(mainSource, /function openBlackboxClipSaveDialog\(\)/)
  assert.match(mainSource, /blackbox-manager:request-save-clip/)
  assert.match(mainSource, /value\.latestClip !== previousStatus\?\.latestClip/)
  assert.match(mainSource, /120000/)
  assert.match(mainSource, /async function deleteBlackboxClip\(fileName\)/)
  assert.doesNotMatch(mainSource, /message: "선택한 클립을 삭제할까요\?"/)
  assert.match(
    mainSource,
    /blackbox-manager:delete-clip[\s\S]*blackboxClipPath\(fileName\)[\s\S]*deleteBlackboxClip\(fileName\)/,
  )
  assert.doesNotMatch(managerSource, /showMessageDialog/)
  assert.match(mainSource, /같은 이름의 클립이 이미 있습니다/)
  assert.match(mainSource, /windowStatePath: join\(directory, "window\.json"\)/)
  assert.match(mainSource, /function readBlackboxManagerSize\(\)/)
  assert.match(mainSource, /function openBlackboxManager\(\)[\s\S]*window\.on\("will-resize"/)
  assert.match(mainSource, /function openBlackboxManager\(\)[\s\S]*window\.on\("resize"/)
  assert.doesNotMatch(mainSource, /blackboxManagerAutomaticBounds/)
  assert.match(mainSource, /blackboxManagerPreferredSize/)
  assert.match(mainSource, /blackboxManagerActivePage/)
  assert.match(mainSource, /if \(page === "extract"\) \{[\s\S]*saveBlackboxManagerSize/)
  assert.match(mainSource, /const desiredViewportHeight = viewportWidth \/ ratio/)
  assert.match(mainSource, /const desiredViewportWidth = viewportHeight \* ratio/)
  assert.match(mainSource, /bounds\.x - \(targetWidth - bounds\.width\) \/ 2/)
  assert.match(managerSource, /grid-template-columns: 290px minmax\(0, 1fr\)/)
  assert.match(managerSource, /\.clip-browser \{[\s\S]*width: 290px;[\s\S]*min-width: 290px;[\s\S]*max-width: 290px;/)
  assert.match(managerSource, /function beginClipRename\(clip, item, nameButton\)/)
  assert.match(managerSource, /function removeClip\(clip\)/)
  assert.match(managerSource, /!selectedClipName && clip === clips\[0\]/)
  assert.match(managerSource, /clipVideo\.addEventListener\("loadedmetadata", fitSavedClipMedia\)/)
  assert.match(managerSource, /document\.body\.dataset\.page !== "clips"/)
  assert.match(managerSource, /preview\.classList\.remove\("media-ready"\)[\s\S]*preview\.clientWidth[\s\S]*preview\.classList\.add\("media-ready"\)/)
  assert.match(managerSource, /window\.requestAnimationFrame\(\(\) => \{\s*window\.requestAnimationFrame\(fitSavedClipMedia\)/)
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
    editorStyle,
    nativeSource,
    viteSource,
  ] = await Promise.all([
    readFile(new URL("blackbox-manager.html", root), "utf8"),
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/blackbox-editor-preload.cjs", root), "utf8"),
    readFile(new URL("blackbox-editor.html", root), "utf8"),
    readFile(new URL("web/blackbox-editor.js", root), "utf8"),
    readFile(new URL("web/blackbox-editor.css", root), "utf8"),
    readFile(new URL("native/recorder-helper/main.cpp", root), "utf8"),
    readFile(new URL("vite.config.mjs", root), "utf8"),
  ])
  assert.match(managerSource, /영상 추출/)
  assert.match(managerSource, /class="extract-frame"[\s\S]*allowfullscreen/)
  assert.match(managerSource, /class="shortcut-input"[\s\S]*readonly/)
  assert.match(managerSource, /최대 녹화 길이[\s\S]*class="max-duration-hours"[\s\S]*value="1"[\s\S]*class="max-duration-minutes"[\s\S]*value="0"[\s\S]*class="max-duration-seconds"[\s\S]*value="0"/)
  assert.match(managerSource, /녹화 용량 한도/)
  assert.match(managerSource, /maxDurationSeconds: normalizeMaximumDurationInputs\(\)/)
  assert.match(managerSource, /function shortcutFromKeyboardEvent\(event\)/)
  assert.match(managerSource, /shortcutInput\.dataset\.accelerator = accelerator/)
  assert.match(managerSource, /Pause: "Pause"/)
  assert.match(managerSource, /function disableShortcut\(\)[\s\S]*shortcutInput\.dataset\.accelerator = ""[\s\S]*사용 안 함/)
  assert.match(mainSource, /closeWindowOnEscape\(window, "blackbox-manager:escape-pressed"\)/)
  assert.match(managerSource, /단축키를 다른 프로그램이 사용 중입니다/)
  assert.match(mainSource, /let activeBlackboxShortcut = null/)
  assert.match(mainSource, /async function refreshBlackboxStorageSummary\(\)/)
  assert.match(mainSource, /"--mode=summary"/)
  assert.match(mainSource, /const storedStatus = !processRunning && blackboxStorageSummary/)
  assert.match(mainSource, /registerBlackboxShortcut\(setting\.shortcut\)/)
  assert.match(mainSource, /nativeBlackboxShortcutVirtualKeys[\s\S]*\["Pause", 0x13\]/)
  assert.match(mainSource, /`--shortcut-vk=\$\{nativeBlackboxShortcutVirtualKeys\.get\(normalized\.shortcut\) \?\? 0\}`/)
  assert.match(mainSource, /if \(line === "SHORTCUT"\) openBlackboxClipSaveDialog\(\)/)
  assert.match(mainSource, /shortcutAccelerator: setting\.shortcut/)
  assert.match(managerSource, /window\.blackboxManager\.editor/)
  assert.match(mainSource, /blackbox-manager:get-editor-session/)
  assert.match(mainSource, /blackbox-manager:set-enabled/)
  assert.match(mainSource, /blackbox-manager:set-track-seconds/)
  assert.match(mainSource, /blackbox-manager:extract/)
  assert.match(mainSource, /title: "블랙박스 영상 추출"[\s\S]*alwaysOnTop: true/)
  assert.match(mainSource, /closeWindowOnEscape\(window\)/)
  assert.match(mainSource, /async function latestCompletedBlackboxAnchor/)
  assert.match(mainSource, /async function latestCompletedBlackboxAnchor\(\)[\s\S]*return Date\.now\(\)/)
  assert.doesNotMatch(mainSource, /flushBlackboxForEditor/)
  assert.match(mainSource, /protocol\.handle\("nogirem-blackbox"/)
  assert.match(mainSource, /async function localVideoResponse/)
  assert.match(mainSource, /"Accept-Ranges": "bytes"/)
  assert.match(mainSource, /status = 206/)
  assert.match(mainSource, /"Content-Range"/)
  assert.match(mainSource, /queueBlackboxControlOperation/)
  assert.match(mainSource, /const videoUrl = `nogirem-blackbox:\/\/editor\//)
  assert.match(mainSource, /"--mode=index"/)
  assert.doesNotMatch(mainSource, /ensureBlackboxEditorTrackMedia/)
  assert.match(editorScript, /segment\.videoUrl/)
  assert.match(preloadSource, /blackbox-editor:set-track-seconds/)
  assert.match(preloadSource, /blackbox-editor:set-enabled/)
  assert.match(preloadSource, /onExtractProgress/)
  assert.match(preloadSource, /blackbox-editor:extract/)
  assert.match(editorSource, /class="gap-policy"/)
  assert.match(editorSource, /건너뛰고 이어붙이기/)
  assert.match(editorSource, /class="playback-speed"/)
  assert.match(editorSource, /class="export-modal" hidden/)
  assert.match(editorSource, /class="gap-policy-field"/)
  assert.match(editorSource, /<h2>클립 저장 설정<\/h2>/)
  assert.match(editorSource, /class="export-name"[^>]*maxlength="120"/)
  assert.match(editorScript, /gapPolicy: gapPolicySelect\.value/)
  assert.match(editorScript, /playbackSpeed: selectedPlaybackSpeed\(\)/)
  assert.match(editorScript, /requestedName: exportNameInput\.value\.trim\(\)/)
  assert.match(editorScript, /function renderExtractProgress\(progress\)/)
  assert.match(editorScript, /extractButton\.textContent = `저장 중 \$\{normalized\}%`/)
  assert.match(editorScript, /extractButton\.textContent = "클립 저장하기"/)
  assert.match(editorScript, /command === "extract-progress"/)
  assert.match(editorStyle, /\.action\.extract\.extracting:disabled/)
  assert.match(editorSource, /class="enable-blackbox"[^>]*hidden>블랙박스 켜기/)
  assert.match(editorScript, /message !== "블랙박스 녹화가 실행 중이 아닙니다"/)
  assert.match(editorScript, /await editorBridge\.setEnabled\(true\)/)
  assert.match(editorScript, /if \(!trackSegments\.length\)[\s\S]*최근 \$\{formatRecordedDuration\(requestedTrackSeconds\)\} 동안 녹화된 영상이 없습니다/)
  assert.match(editorScript, /editor\.className = "editor ready no-recording"/)
  assert.match(editorStyle, /\.editor\.no-recording \.empty-state \{[\s\S]*display: grid;/)
  assert.match(mainSource, /videoUrl: playbackSegments\[0\]\?\.videoUrl \?\? ""/)
  assert.doesNotMatch(mainSource, /if \(!playbackSegments\.length\) throw/)
  assert.match(nativeSource, /if \(chunks\.empty\(\) && mode == L"track"\)/)
  assert.match(nativeSource, /rawTimelineEnd <= 0\.0 \|\| rawTimelineStart >=/)
  assert.match(mainSource, /mediaStartSeconds: Number\(chunk\.mediaStartSeconds\) \|\| 0/)
  assert.match(editorScript, /async function retryTrackWhenRecordingStarts\(value\)/)
  assert.match(editorScript, /!value\?\.running[\s\S]*!value\?\.recording/)
  assert.match(editorScript, /void retryTrackWhenRecordingStarts\(event\.data\.value\)/)
  assert.match(managerSource, /running: value\.running,[\s\S]*recording: value\.recording/)
  assert.match(editorSource, /class="compact-button add-time"[^>]*>\+5분</)
  assert.match(editorSource, /class="compact-button direct-time"[^>]*>\+\+</)
  assert.match(editorSource, /track-length-buttons[\s\S]*direct-time[\s\S]*add-time/)
  assert.match(editorSource, /블랙박스 조회 길이/)
  assert.match(editorSource, /id="track-hours"[^>]*value="0"[\s\S]*>시간<\/span>/)
  assert.match(editorSource, /id="track-minutes"[^>]*value="15"[\s\S]*>분<\/span>/)
  assert.match(editorSource, /id="track-seconds"[\s\S]*>초<\/span>/)
  assert.match(editorSource, /class="selection-guide"/)
  assert.match(editorSource, /class="selection-handle start"/)
  assert.match(editorSource, /class="selection-handle end"/)
  assert.match(editorSource, /class="track-gaps"/)
  assert.match(
    editorSource,
    /<footer class="actions">[\s\S]*class="track-status"[\s\S]*track-duration[\s\S]*track-usage[\s\S]*클립 저장하기/,
  )
  assert.doesNotMatch(editorSource, /preview-range/)
  assert.match(editorSource, /id="extract-hours"[^>]*value="0"[\s\S]*>시간<\/span>/)
  assert.match(editorSource, /id="extract-minutes"[^>]*value="1"[\s\S]*>분<\/span>/)
  assert.match(editorSource, /id="extract-seconds"[^>]*value="0"[\s\S]*>초<\/span>[\s\S]*class="range-time"/)
  assert.match(editorScript, /let selectionDuration = 60/)
  assert.match(editorScript, /selectionDuration = Math\.min\(60, timelineDuration\)/)
  assert.match(editorScript, /현재 \$\{\(bytesUsed \/ 1024 \*\* 3\)\.toFixed\(1\)\} \/ 최대 \$\{capacityGb\}GB/)
  assert.match(editorScript, /\$\{formatRecordedDuration\(durationSeconds\)\} 녹화됨/)
  assert.match(editorStyle, /grid-template-areas:[\s\S]*"timeline timeline timeline"[\s\S]*"controls playback extract"/)
  assert.match(editorStyle, /\.track-length-buttons \{[\s\S]*grid-template-columns: 54px 54px;/)
  assert.match(editorStyle, /\.track-status \{[\s\S]*display: flex;[\s\S]*font-size: 12px;/)
  assert.match(editorStyle, /\.extract-duration \{[\s\S]*display: flex;/)
  assert.match(editorScript, /requestedTrackSeconds \+ 300/)
  assert.match(editorScript, /function normalizeTrackLengthInputs\(\)/)
  assert.match(editorScript, /function normalizeExtractDurationInputs\(\)/)
  assert.match(editorStyle, /grid-template-columns: 114px auto minmax\(0, 1fr\)/)
  assert.match(managerSource, /\.settings \{[\s\S]*width: min\(480px, 100%\)/)
  assert.match(managerSource, /window\.blackboxManager\.setPage\(page\)/)
  assert.match(mainSource, /function setBlackboxManagerPage\(page\)/)
  assert.match(mainSource, /compact \? 540/)
  assert.match(mainSource, /compact \? 760/)
  assert.match(editorScript, /function fitCurrentMedia\(\)/)
  assert.match(editorScript, /page: "extract"/)
  assert.match(managerSource, /page: "clips"/)
  assert.match(editorScript, /function renderManagerStatus\(value\)/)
  assert.match(editorScript, /hours \* 3600 \+ minutes \* 60 \+ seconds/)
  assert.match(editorScript, /Math\.floor\(normalized \/ 3600\)/)
  assert.match(editorScript, /Math\.floor\(normalized % 3600 \/ 60\)/)
  assert.match(editorScript, /normalized % 60/)
  assert.match(editorScript, /requestParent\("getSession"\)/)
  assert.match(editorScript, /let requestedTrackSeconds = 900/)
  assert.match(
    editorScript,
    /document\.addEventListener\("pointerdown"[\s\S]*!directLengthForm\.contains\(event\.target\)[\s\S]*directLengthForm\.classList\.remove\("visible"\)/,
  )
  assert.match(mainSource, /return createBlackboxEditorTrack\(session, 900\)/)
  assert.match(mainSource, /if \(session\.preparePromise === preparation\) session\.preparePromise = null/)
  assert.match(editorScript, /else if \(event\.code === "Space"\)/)
  assert.match(editorScript, /function beginTimelineInteraction\(event\)/)
  assert.match(editorScript, /function seekTimelineBy\(seconds\)/)
  assert.match(editorScript, /event\.code === "ArrowLeft" \|\| event\.code === "ArrowRight"/)
  assert.match(editorScript, /seekTimelineBy\(event\.code === "ArrowLeft" \? -5 : 5\)/)
  assert.match(managerSource, /ArrowLeft: "seek-backward"[\s\S]*ArrowRight: "seek-forward"[\s\S]*Enter: "toggle-fullscreen"/)
  assert.match(editorScript, /event\.data\.command === "seek-backward"[\s\S]*seekTimelineBy\(-5\)/)
  assert.match(editorScript, /event\.data\.command === "toggle-fullscreen"[\s\S]*togglePreviewFullscreen\(\)/)
  assert.match(editorScript, /async function togglePreviewFullscreen\(\)/)
  assert.match(editorScript, /await preview\.requestFullscreen\(\)/)
  assert.match(editorStyle, /body\.embedded \.preview:fullscreen/)
  assert.doesNotMatch(editorScript, /resetToSelection/)
  assert.match(editorScript, /timelineCursor = playbackEnd[\s\S]*stopTimelinePlayback\(\)[\s\S]*syncPreviewToTimeline\(\)/)
  assert.match(editorScript, /mode = "resize-start"/)
  assert.match(editorScript, /mode = "resize-end"/)
  assert.match(editorScript, /mode === "resize-end"[\s\S]*selectionStart \+ selectionDuration/)
  assert.match(editorScript, /if \(mode === "seek" \|\| mode === "pending-move"\) seekFromPointer\(event\)/)
  assert.match(editorScript, /function setTimelineCursor\(time\)/)
  assert.match(editorScript, /gapPreview\.hidden = Boolean\(segment\)/)
  assert.match(editorScript, /selectionStart = Math\.max\(\s*0,[\s\S]*timelineDuration - selectionDuration/)
  assert.match(editorSource, /class="gap-preview" hidden/)
  assert.match(editorStyle, /body\.embedded \.gap-preview \{\s*inset: 0;/)
  assert.match(editorStyle, /\.gap-preview \{[\s\S]*?inset: 0;[\s\S]*?background: #000;/)
  assert.match(editorStyle, /\.track-gap \{[\s\S]*background: #000;/)
  assert.match(editorScript, /function renderTrackGaps\(\)/)
  assert.match(editorScript, /element\.className = "track-gap"/)
  assert.match(mainSource, /return createBlackboxEditorTrack\(session, 900\)/)
  assert.match(mainSource, /const metadataOutput = await runRecorderUtility/)
  assert.match(mainSource, /Array\.isArray\(metadata\.gaps\)/)
  assert.match(mainSource, /Array\.isArray\(metadata\.segments\)/)
  assert.match(mainSource, /session\.trackMediaSeconds/)
  assert.match(mainSource, /function buildBlackboxExtractionPieces\(/)
  assert.match(mainSource, /gapPolicy === "black"/)
  assert.match(mainSource, /value\?\.gapPolicy === "skip"/)
  assert.match(mainSource, /--speed-milli=/)
  assert.match(mainSource, /blackbox-editor:extract-progress/)
  assert.match(mainSource, /type: "black"/)
  assert.match(mainSource, /"--mode=compose"/)
  assert.match(mainSource, /"--mode=compose"[\s\S]*`--ring-path=\$\{join\(getBlackboxPaths\(\)\.storagePath, "Ring"\)\}`/)
  assert.doesNotMatch(mainSource, /"--mode=compose"[\s\S]{0,200}`--input=/)
  assert.doesNotMatch(editorScript, /previewSelection/)
  assert.match(editorScript, /extractButton\.addEventListener\("click", openExportModal\)/)
  assert.match(editorScript, /exportDialog\.addEventListener\("submit"/)
  assert.match(editorScript, /function selectionContainsGap\(\)/)
  assert.match(editorScript, /const overlapStart = Math\.max\(selectionStart, gapStart\)/)
  assert.match(editorScript, /overlapEnd - overlapStart >= 1/)
  assert.match(editorScript, /gapPolicyField\.hidden = !selectionContainsGap\(\)/)
  assert.match(editorScript, /suggestClipName: \(\) => requestParent\("suggestClipName"\)/)
  assert.match(mainSource, /function normalizeBlackboxClipFileName\(requestedName\)/)
  assert.match(mainSource, /requestedFileName \|\| `마비노기-추출-/)
  assert.match(mainSource, /function suggestBlackboxClipName\(\)/)
  assert.match(mainSource, /blackbox-manager:suggest-clip-name/)
  assert.match(editorScript, /setNotice\(`\$\{result\.fileName\} 저장 완료`\)[\s\S]*await editorBridge\.showOutput\(result\.outputPath\)/)
  assert.match(nativeSource, /mode == L"track"/)
  assert.match(nativeSource, /mode == L"compose"/)
  assert.match(nativeSource, /mode == L"extract"/)
  assert.match(nativeSource, /runUtilityMode[\s\S]*PROCESS_MODE_BACKGROUND_BEGIN/)
  assert.match(nativeSource, /status\.fps = options\.fps;[\s\S]*BELOW_NORMAL_PRIORITY_CLASS/)
  assert.match(mainSource, /async function getRecorderAffinityArgument\(\)/)
  assert.match(mainSource, /const isolatedRecorderMask = allocation\.backgroundMask & ~latencyMask/)
  assert.match(mainSource, /affinityArgument = await getRecorderAffinityArgument\(\)/)
  assert.match(nativeSource, /arguments\.find\(L"affinity-mask"\)/)
  assert.match(nativeSource, /options\.shortcutVirtualKey = integerArgument/)
  assert.match(nativeSource, /GetAsyncKeyState\(options\.shortcutVirtualKey\)/)
  assert.match(nativeSource, /std::cout << "SHORTCUT\\n" << std::flush/)
  assert.match(nativeSource, /SetProcessAffinityMask\(GetCurrentProcess\(\), static_cast<DWORD_PTR>\(mask\)\)/)
  assert.match(nativeSource, /SetGPUThreadPriority\(-2\)/)
  assert.match(nativeSource, /void requireHardwareVideoEncoder\(/)
  assert.match(nativeSource, /MFT_ENUM_FLAG_HARDWARE \| MFT_ENUM_FLAG_SORTANDFILTER/)
  assert.match(nativeSource, /std::unordered_map<[\s\S]*ID3D11VideoProcessorInputView/)
  assert.match(nativeSource, /captureRetryDelay = std::min\(captureRetryDelay \* 2, 5000ms\)/)
  assert.match(
    nativeSource,
    /capturedAt \+ 1ms < \*nextFrameAt_[\s\S]*\*nextFrameAt_ \+= minimumFrameInterval_[\s\S]*encoder_\.enqueue/,
  )
  assert.match(nativeSource, /class RingStorageIndex/)
  assert.match(mainSource, /import \{ access, copyFile,[^\n]+ \} from "node:fs\/promises"/)
  assert.match(nativeSource, /double completedChunkDurationSeconds\(const fs::path& path\)/)
  assert.match(nativeSource, /mode == L"summary"/)
  assert.match(nativeSource, /arguments\.at\(L"mode"\) == L"summary"/)
  assert.match(nativeSource, /durationSeconds_ = totalDuration;/)
  assert.match(nativeSource, /ringStorage\.durationSeconds\(\) \+ encoder\.currentChunkDurationSeconds\(\)/)
  assert.doesNotMatch(nativeSource, /estimatedCompletedSeconds/)
  assert.match(mainSource, /`--max-duration-seconds=\$\{normalized\.maxDurationSeconds\}`/)
  assert.match(nativeSource, /options\.maxDurationSeconds = integerArgument\(/)
  assert.match(nativeSource, /const auto reserve = Gigabyte;/)
  assert.match(nativeSource, /newestStarted - chunks_\.front\(\)\.started\s*>= maxDurationMilliseconds/)
  assert.match(nativeSource, /std::atomic_uint64_t droppedFrames = 0/)
  assert.match(mainSource, /metricsPath: join\(directory, "recorder-metrics\.log"\)/)
  assert.match(
    mainSource,
    /spawn\(recorderHelperPath,[\s\S]*`--metrics-path=\$\{paths\.metricsPath\}`/,
  )
  assert.match(nativeSource, /class RecorderMetrics/)
  assert.match(nativeSource, /if \(now - lastMetricsWrite >= 1min\)/)
  assert.match(nativeSource, /GetProcessMemoryInfo\(/)
  assert.match(nativeSource, /QueryVideoMemoryInfo\(/)
  assert.match(nativeSource, /maximumVideoQueueDepth/)
  assert.match(nativeSource, /maximumAudioQueueFrames/)
  assert.match(nativeSource, /const auto bytesUsed = ringStorage_\.publish\(/)
  assert.match(nativeSource, /status\.bytesUsed = ringStorage\.bytesUsed\(\)/)
  assert.doesNotMatch(nativeSource, /status\.bytesUsed = directoryBytes/)
  assert.doesNotMatch(nativeSource, /pruneRing\(/)
  assert.match(nativeSource, /const LONGLONG audioRequestedStart = requestedDuration == LLONG_MAX\s*\? 0\s*:\s*requestedStart;/)
  assert.match(
    nativeSource,
    /piece\.start,[\s\S]*piece\.duration,[\s\S]*false,[\s\S]*1\.0/,
  )
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
  assert.doesNotMatch(nativeSource, /combinedDecodedAudioDuration/)
  assert.match(nativeSource, /const LONGLONG fileStart = outputTime;/)
  assert.match(nativeSource, /const LONGLONG globalTime = fileStart \+ relativeTime;/)
  assert.match(nativeSource, /relativeTime \+ 2500000ll < fileAudioTime/)
  assert.match(nativeSource, /const auto mappedOutputTime = std::max<LONGLONG>/)
  assert.match(nativeSource, /sample->DeleteItem\(MFSampleExtension_DecodeTimestamp\)/)
  assert.match(nativeSource, /TrackTimelineMetadata trackTimelineMetadata\(/)
  assert.match(nativeSource, /continuityToleranceSeconds = 1\.5/)
  assert.match(nativeSource, /if \(joinsPreviousSegment\) \{\s*segments\.back\(\)\.duration \+= visibleDuration;/)
  assert.match(nativeSource, /\\"segments\\":\[/)
  assert.match(nativeSource, /\\"gaps\\":\[/)
  assert.match(nativeSource, /void createBlackVideo\(/)
  assert.match(nativeSource, /void composeExtraction\(/)
  assert.match(nativeSource, /const std::vector<fs::path>& inputPaths/)
  assert.match(nativeSource, /pieces\.size\(\) == 1[\s\S]*std::abs\(playbackRate - 1\.0\) < 0\.001/)
  assert.match(nativeSource, /outputTime \+ fileDuration <= effectiveStart/)
  assert.match(nativeSource, /if \(onProgress\) onProgress\(fileIndex \+ 1, inputs\.size\(\)\);/)
  assert.match(nativeSource, /ComPtr<IMFSample> retimePcmSample\(/)
  assert.match(nativeSource, /LONGLONG maximumInputDuration = LLONG_MAX/)
  assert.match(nativeSource, /const auto availableInputDuration = std::min\(\s*fileEnd,\s*audioRequestedEnd\s*\) - globalTime;/)
  assert.match(nativeSource, /wideInteger\(arguments, L"speed-milli", 1000\)/)
  assert.match(nativeSource, /std::cerr << "PROGRESS "/)
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
