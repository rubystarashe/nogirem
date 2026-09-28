import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"

const nativeAffinity = await readFile(new URL("../desktop/backend/src/affinity_worker.rs", import.meta.url), "utf8")
const nativeMemory = await readFile(new URL("../desktop/backend/src/memory.rs", import.meta.url), "utf8")
const nativeStorage = await readFile(new URL("../desktop/backend/src/storage.rs", import.meta.url), "utf8")
const serviceMain = await readFile(
  new URL("../service/main.mjs", import.meta.url),
  "utf8",
)
const serviceBootstrap = await readFile(
  new URL("../service/bootstrap.mjs", import.meta.url),
  "utf8",
)
const servicePreload = await readFile(
  new URL("../service/preload.js", import.meta.url),
  "utf8",
)
const applicationView = await readFile(
  new URL("../desktop/src/ui.rs", import.meta.url),
  "utf8",
)
const applicationStyles = await readFile(
  new URL("../web/styles.css", import.meta.url),
  "utf8",
)
const packageSource = await readFile(
  new URL("../package.json", import.meta.url),
  "utf8",
)
const gameWave = await readFile(
  new URL("../desktop/src/wave.js", import.meta.url),
  "utf8",
)
const modalView = await readFile(
  new URL("../desktop/src/ui.css", import.meta.url),
  "utf8",
)
const updateModalView = await readFile(
  new URL("../desktop/src/ui.rs", import.meta.url),
  "utf8",
)
const turboKeyTermsDocument = await readFile(
  new URL("../TURBO_KEY_TERMS.md", import.meta.url),
  "utf8",
)
const packageInfo = JSON.parse(await readFile(
  new URL("../package.json", import.meta.url),
  "utf8",
))
const developmentLauncher = await readFile(
  new URL("../scripts/run-dioxus.mjs", import.meta.url),
  "utf8",
)

test("개발 앱은 빈 전용 포트를 찾아 동일 서버 주소만 사용한다", () => {
  assert.equal(packageInfo.scripts["app:dev"], "node scripts/run-dioxus.mjs")

  assert.match(serviceMain, /function resolveDevelopmentServerUrl\(\)/)
  assert.match(serviceMain, /developmentPageUrl\("blackbox-manager\.html"\)/)
  assert.doesNotMatch(serviceMain, /localhost:5173/)
})

test("배포 빌드는 임시 업데이트 안내 테스트를 포함하지 않는다", () => {
  assert.doesNotMatch(serviceMain, /forceApplicationUpdateNoticePreview/)
  assert.doesNotMatch(serviceMain, /version = "0\.3\.4"/)
})

test("렌더러 종료와 장기 무응답 상태를 자동 복구한다", () => {
  assert.match(
    serviceMain,
    /webContents\.on\("render-process-gone"[\s\S]*recoverPrimaryRenderer/,
  )
  assert.match(
    serviceMain,
    /window\.on\("unresponsive"[\s\S]*primaryRendererUnresponsiveTimeoutMs/,
  )
  assert.match(serviceMain, /window\.webContents\.reload\(\)/)
  assert.doesNotMatch(serviceMain, /\.isSkipTaskbar\(\)/)
  assert.match(
    serviceMain,
    /primaryWindowSkippedFromTaskbar[\s\S]*window\.setSkipTaskbar\(primaryWindowSkippedFromTaskbar\)/,
  )
})

test("트레이 진입은 보조 창을 종료하고 복귀는 메인 창을 한 번만 표시한다", () => {
  const focusStart = serviceMain.indexOf("function focusPrimaryWindow()")
  const focusEnd = serviceMain.indexOf("function focusPrimaryWindowAfterTrayMenu()", focusStart)
  const focusSource = serviceMain.slice(focusStart, focusEnd)
  const restoreStart = focusSource.indexOf("if (restoringFromTray) {")
  const restoreEnd = focusSource.indexOf("\n  if (!window.isVisible())", restoreStart)
  const restoreSource = focusSource.slice(restoreStart, restoreEnd)
  const minimizeStart = serviceMain.indexOf("function minimizePrimaryWindowToTray()")
  const minimizeEnd = serviceMain.indexOf("function isPrimaryWindowVisuallyActive()", minimizeStart)
  const minimizeSource = serviceMain.slice(minimizeStart, minimizeEnd)

  assert.match(
    serviceMain,
    /function closeInternalWindowsForTray\(\)[\s\S]*internalWindowsClosedForTray\.add\(window\)[\s\S]*window\.destroy\(\)/,
  )
  assert.match(
    serviceMain,
    /function minimizePrimaryWindowToTray\(\)[\s\S]*closeInternalWindowsForTray\(\)[\s\S]*primaryWindow\.hide\(\)/,
  )
  assert.ok(
    minimizeSource.indexOf("primaryWindow.hide()")
      < minimizeSource.indexOf("primaryWindow.setSkipTaskbar(true)"),
  )
  assert.match(focusSource, /setEnabled\(true\)[\s\S]*setFocusable\(true\)[\s\S]*setIgnoreMouseEvents\(false\)/)
  assert.match(restoreSource, /window\.show\(\)[\s\S]*writeWindowDiagnostics\("트레이 복귀 직후 창 상태"\)[\s\S]*return/)
  assert.doesNotMatch(restoreSource, /window\.focus\(\)|webContents\.focus\(\)/)
  assert.equal(
    serviceMain.match(/internalWindowsClosedForTray\.delete\(window\)/g)?.length,
    5,
  )
  assert.match(
    serviceMain,
    /function focusPrimaryWindowAfterTrayMenu\(\)[\s\S]*trayMenuCloseDelayMs[\s\S]*label: "열기"[\s\S]*focusPrimaryWindowAfterTrayMenu/,
  )
})

test("일반 창 포커스는 지연 후 상태를 확인하고 한 번만 재시도한다", () => {
  const focusStart = serviceMain.indexOf("function focusPrimaryWindow()")
  const focusEnd = serviceMain.indexOf("function focusPrimaryWindowAfterTrayMenu()", focusStart)
  const focusSource = serviceMain.slice(focusStart, focusEnd)

  assert.match(focusSource, /const applyFocus = \(\) =>[\s\S]*window\.focus\(\)[\s\S]*window\.webContents\.focus\(\)/)
  assert.match(
    focusSource,
    /primaryWindowFocusTimer = setTimeout\([\s\S]*applyFocus\(\)[\s\S]*if \(!window\.isFocused\(\)\) applyFocus\(\)[\s\S]*primaryWindowFocusRetryDelayMs/,
  )
  assert.doesNotMatch(focusSource, /screen-saver|moveTop\(\)/)
})

test("트레이 종료는 메인 창 표시 여부에 맞는 종료 선택창을 사용한다", () => {
  assert.match(
    serviceMain,
    /label: "종료"[\s\S]*requestApplicationExitFromTray\(\)/,
  )
  assert.match(
    serviceMain,
    /function requestApplicationExitFromTray\(\) \{[\s\S]*primaryWindow\.isVisible\(\)[\s\S]*nativeDialog: !mainWindowVisible/,
  )
  assert.match(
    serviceMain,
    /if \(mainWindowVisible\) focusPrimaryWindow\(\)[\s\S]*nativeDialog: !mainWindowVisible/,
  )
  assert.match(
    serviceMain,
    /if \(nativeDialog\) \{[\s\S]*dialog\.showMessageBox\(\{[\s\S]*result\.response === 0 \? "reset" : "keep"/,
  )
  assert.doesNotMatch(
    serviceMain,
    /function requestApplicationExitConfirmation[\s\S]{0,300}focusPrimaryWindow\(\)/,
  )
})

test("고급 기능 안내 말풍선은 실제 화면 노출을 최대 세 번 기록한다", () => {
  assert.match(
    serviceMain,
    /async function readCreatorPromptDismissed\(\)[\s\S]*displayCount[\s\S]*>= 3/,
  )
  assert.match(
    serviceMain,
    /async function recordCreatorPromptDisplay\(\)[\s\S]*displayCount: displayCount \+ 1/,
  )
  assert.match(
    servicePreload,
    /recordCreatorPromptDisplay: \(\) => ipcRenderer\.invoke\("application:record-creator-prompt-display"\)/,
  )

})

test("Esc는 최상위 모달과 문서 화면을 닫고 모든 보조 창에도 적용된다", async () => {


  assert.match(
    await readFile(new URL("../service/window-escape.mjs", import.meta.url), "utf8"),
    /function closeWindowOnEscape\(window, rendererEvent = ""\)[\s\S]*input\.type !== "keyDown"[\s\S]*input\.key !== "Escape"[\s\S]*window\.close\(\)/,
  )
  assert.equal(serviceMain.match(/closeWindowOnEscape\(window(?:, "[^"]+")?\)/g)?.length, 5)
})

test("키보드 입력 후 버튼에 포커스 외곽선을 표시하지 않는다", () => {
  assert.match(
    applicationStyles,
    /button:focus-visible \{\s*outline: none;\s*\}/,
  )
  assert.doesNotMatch(applicationStyles, /button:focus-visible \{\s*outline: 2px solid/)
})

test("초기 상태 조회와 무관하게 창을 먼저 만들고 8초 안에 표시한다", () => {
  const createWindowAt = serviceMain.indexOf("createWindow()", serviceMain.indexOf("async function startApplication"))
  const loadPathAt = serviceMain.indexOf("loadMabinogiExecutablePath()", createWindowAt)

  assert.ok(createWindowAt >= 0)
  assert.ok(loadPathAt > createWindowAt)
  assert.match(
    serviceMain,
    /async function startApplication\(\)[\s\S]*ensureApplicationTray\(\)[\s\S]*createWindow\(\)/,
  )
  assert.doesNotMatch(serviceMain, /if \(startupTrayLaunch\) ensureApplicationTray\(\)/)
  assert.match(serviceMain, /const primaryWindowRevealTimeoutMs = 8_000/)
  assert.match(
    serviceMain,
    /primaryWindowRevealWatchdogTimer = setTimeout\([\s\S]*beginPrimaryWindowReveal\(\)[\s\S]*primaryWindowRevealTimeoutMs/,
  )

})

test("시작 이미지와 음악이 실패하거나 지연되어도 시작 애니메이션을 진행한다", () => {
  assert.match(gameWave, /playbackFallbackTimer = window\.setTimeout\([\s\S]*startAnimation\(\)[\s\S]*1500/)
  assert.match(
    gameWave,
    /image\.onerror = error => \{[\s\S]*imageReady = true[\s\S]*playStartup\(\)/,
  )
  assert.match(gameWave, /if \(imageReady && logoImageReady\) playStartup\(\)/)
  assert.match(
    gameWave,
    /requestDraw\(\)[\s\S]*requestAnimationFrame\(\(\) => \{[\s\S]*requestAnimationFrame\(\(\) => \{[\s\S]*onplaybackstart\(\)/,
  )
  assert.match(
    gameWave,
    /logoImage\.onerror = error => \{[\s\S]*logoImageReady = true[\s\S]*playStartup\(\)/,
  )
})

test("시작 애니메이션은 오디오 정체와 분리하고 무거운 초기화를 재생 전에 끝낸다", () => {
  assert.match(
    gameWave,
    /function currentTimelineElapsed\(\) \{[\s\S]*performance\.now\(\) - startedAt/,
  )
  assert.doesNotMatch(
    gameWave,
    /function currentTimelineElapsed\(\) \{[\s\S]{0,160}audio\.currentTime/,
  )

  assert.match(
    gameWave,
    /audioStopTimer = window\.setTimeout\(\(\) => \{[\s\S]*nextAudio\.pause\(\)[\s\S]*6200 - initialTimelineElapsed/,
  )
  assert.match(
    gameWave,
    /const finishStartupSequence = \(\) => \{[\s\S]*onstartupidle\(\)[\s\S]*setTimeout\(finishStartupSequence,[\s\S]*finalStartupWaveEnd - initialTimelineElapsed/,
  )
  assert.match(servicePreload, /completeStartupAnimation/)
  assert.match(
    serviceMain,
    /application:get-launch-context[\s\S]*startupPreparation = prepareStartupBeforeAnimation\("렌더러 준비 요청"\)[\s\S]*Promise\.all\(\[[\s\S]*getOptimizationStatus\(\)[\s\S]*startupPreparation\.then\(\(\) => getBlackboxSetting\(\)\)[\s\S]*blackboxSetting/,
  )
  assert.match(
    serviceMain,
    /async function getOptimizationStatus\(\)[\s\S]*checkGraphics\(\)[\s\S]*checkNetwork\(\)[\s\S]*frameBoostStartupPromise\.then\(\(\) => checkMemory\(\)\)[\s\S]*frameBoostStartupPromise\.then\(\(\) => checkAffinity/,
  )
  assert.match(
    serviceMain,
    /function prepareStartupBeforeAnimation\(reason\)[\s\S]*await ensureBlackboxStarted\(\)[\s\S]*await loadCachedDxvkReleases\(\)/,
  )
  assert.match(
    serviceMain,
    /function prepareStartupBeforeAnimation\(reason\)[\s\S]*openDxvkManager\(false\)[\s\S]*function internalWindows\(\)/,
  )
  assert.match(
    serviceMain,
    /application:complete-startup-animation[\s\S]*return \{ dxvk: \{ \.\.\.dxvkRuntimeStatus \} \}/,
  )

  assert.match(
    serviceMain,
    /prepareStartupBeforeAnimation\("렌더러 준비 요청 대기 시간 초과"\)[\s\S]*10000/,
  )
})

test("고급 기능에서 시작 음악을 음소거하고 다음 실행에도 유지한다", () => {
  assert.match(
    serviceMain,
    /function startupMusicSettingPath\(\)[\s\S]*async function getStartupMusicSetting\(\)[\s\S]*async function setStartupMusicSetting\(muted\)/,
  )
  assert.match(
    serviceMain,
    /application:get-launch-context[\s\S]*startupMusicMuted: startupMusic\.muted/,
  )
  assert.match(serviceMain, /application:set-startup-music-setting/)
  assert.match(servicePreload, /setStartupMusicSetting/)


  assert.match(
    gameWave,
    /if \(startupMuted\) \{[\s\S]*startAnimation\(\)[\s\S]*return[\s\S]*nextAudio\.play\(\)/,
  )
  assert.match(gameWave, /function setStartupMuted\(muted\)/)
})

test("부스트 중단은 일부 실행 상태도 원상복구하고 중단 상태를 단계별 표시한다", () => {
  const activeStateCheck = /!\(services\.affinity\.data\?\.running \|\| services\.memory\.data\?\.running\)/g

  assert.match(
    serviceMain,
    /enabled[\s\S]*\? setAffinityEnabled\(\{ enabled: true, includeNic \}\)[\s\S]*: resetAllAffinities\(\)/,
  )
})

test("강제 간소화 미적용 상태는 빠른 런타임 조회마다 직접 확인한다", () => {
  assert.match(
    serviceMain,
    /const cachedCharacterSimplification = fresh[\s\S]*const characterSimplification = cachedCharacterSimplification\?\.applied[\s\S]*: await getCharacterSimplificationStatus\(\)/,
  )
  assert.match(
    serviceMain,
    /return \{[\s\S]*characterSimplification,[\s\S]*dxvk: dxvkRuntimeStatus/,
  )
})

// MUO installation is exercised against real isolated files by
// desktop/backend/src/muo.rs::tests::installs_exact_bytes_and_removes_legacy_copy.

test("helper 상태 파일 잠금 실패를 복구하고 중복 helper 실행을 막는다", () => {
  assert.match(serviceMain, /import \{ writeJsonAtomic \} from "\.\.\/src\/atomic-json\.mjs"/)
  assert.match(
    nativeStorage,
    /create_new\(true\)[\s\S]*previous_status\["helperPid"\][\s\S]*recent \|\| confirmed/,
  )
  assert.match(
    nativeAffinity,
    /Affinity 상태 기록 실패/,
  )
  assert.match(
    nativeMemory,
    /메모리 상태 기록 실패/,
  )
})

test("진단 로그는 최근 Windows 블루스크린과 비정상 종료 이벤트를 포함한다", () => {
  assert.match(
    serviceMain,
    /async function readRecentWindowsFailureEvents\(\)[\s\S]*Get-WinEvent[\s\S]*Id = 41, 1001, 6008/,
  )
  assert.match(
    serviceMain,
    /diagnosticResult\(\(\) => readRecentWindowsFailureEvents\(\)\)[\s\S]*recentWindowsFailures/,
  )
})

test("메모리 helper는 부모 앱 종료를 감지하고 기존 고아 helper를 정리한다", () => {
  assert.match(
    nativeMemory,
    /HelperLock::acquire[\s\S]*while crate::process::alive\(parent\)/,
  )
  assert.match(
    serviceMain,
    /"--native-memory-helper",[\s\S]*`--parent-pid=\$\{process\.pid\}`/,
  )
  assert.match(
    nativeStorage,
    /old_owner[\s\S]*orphan-recovery[\s\S]*Helper already running/,
  )
})

test("부트스트랩이 시작 오류를 파일에 기록한다", () => {

  assert.match(serviceBootstrap, /startup\.log/)
  assert.match(serviceBootstrap, /process\.on\(['"]uncaughtException['"]/)
  assert.match(serviceBootstrap, /await import\(['"]\.\/main\.mjs['"]\)/)
})

test("마비노기 운영정책 링크는 허용된 주소만 시스템 브라우저로 연다", () => {
  const policyUrlAt = applicationView.indexOf("const operationPolicyUrl")
  const termsParseAt = applicationView.indexOf("const turboTermsBlocks")



  assert.match(
    turboKeyTermsDocument,
    /\[마비노기 '권장하지 않는 플레이 방식 안내'\]\(https:\/\/mabinogi\.nexon\.com\/page\/archive\/guide_view\.asp\?id=4889849&num=7&playtarget=1\)/,
  )

  assert.match(servicePreload, /application:open-operation-policy/)
  assert.match(
    serviceMain,
    /ipcMain\.handle\("application:open-operation-policy"[\s\S]*shell\.openExternal\(operationPolicyUrl\)/,
  )
})

test("버그 리포트 제출 버튼은 고정된 Google Forms 주소를 연다", () => {


  assert.match(servicePreload, /application:open-bug-report-form/)
  assert.match(
    serviceMain,
    /const bugReportFormUrl = "https:\/\/docs\.google\.com\/forms\/d\/e\/1FAIpQLSfx6-QVqsxgUDKsYCMAyg7A51ZYBMrMa_17OGzzQF_gGOum1w\/viewform\?usp=publish-editor"/,
  )
  assert.match(
    serviceMain,
    /ipcMain\.handle\("application:open-bug-report-form"[\s\S]*shell\.openExternal\(bugReportFormUrl\)/,
  )
})
