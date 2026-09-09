import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"

const electronMain = await readFile(
  new URL("../electron/main.mjs", import.meta.url),
  "utf8",
)
const electronBootstrap = await readFile(
  new URL("../electron/bootstrap.mjs", import.meta.url),
  "utf8",
)
const electronPreload = await readFile(
  new URL("../electron/preload.cjs", import.meta.url),
  "utf8",
)
const applicationView = await readFile(
  new URL("../web/App.svelte", import.meta.url),
  "utf8",
)
const applicationStyles = await readFile(
  new URL("../web/styles.css", import.meta.url),
  "utf8",
)
const gameWave = await readFile(
  new URL("../web/GameWave.svelte", import.meta.url),
  "utf8",
)
const modalView = await readFile(
  new URL("../web/Modal.svelte", import.meta.url),
  "utf8",
)
const updateModalView = await readFile(
  new URL("../web/UpdatePreviewModal.svelte", import.meta.url),
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

test("업데이트 다운로드가 45초간 멈추면 입력 차단 상태를 해제한다", () => {
  assert.match(electronMain, /const applicationUpdateStallTimeoutMs = 45_000/)
  assert.match(
    electronMain,
    /function armApplicationUpdateStallTimer\(\)[\s\S]*phase: "error"[\s\S]*UpdateDownloadStalled/,
  )
  assert.match(
    electronMain,
    /autoUpdater\.on\("download-progress"[\s\S]*armApplicationUpdateStallTimer\(\)/,
  )
})

test("새 앱 버전은 주기적으로 확인하고 메인 문구와 우측 하단 오버레이로 알린다", () => {
  assert.match(
    electronMain,
    /applicationUpdateCheckTimer = setInterval\([\s\S]*4 \* 60 \* 60 \* 1000/,
  )
  assert.match(
    electronMain,
    /autoUpdater\.on\("update-available"[\s\S]*phase: "available"[\s\S]*showApplicationUpdateNotification\(version\)/,
  )
  assert.match(
    electronMain,
    /function showApplicationUpdateNotification[\s\S]*workArea\.height - height - 24[\s\S]*setIgnoreMouseEvents\(true,[\s\S]*setAlwaysOnTop\(true, "screen-saver", 1\)/,
  )
  assert.match(
    electronMain,
    /마비노기 렘 부스터 새 버전 업데이트가 가능합니다/,
  )
  assert.match(
    applicationView,
    /applicationUpdateAvailable[\s\S]*class:update-available[\s\S]*새 버전 출시/,
  )
  assert.match(
    applicationStyles,
    /\.app-version\.update-available \{[\s\S]*background: #ffd400;[\s\S]*font-weight: 900;/,
  )
})

test("임시 업데이트 안내 테스트는 앱 시작 직후 활성화된다", () => {
  assert.match(electronMain, /const forceApplicationUpdateNoticePreview = true/)
  assert.match(
    electronMain,
    /if \(forceApplicationUpdateNoticePreview\)[\s\S]*version = "0\.3\.4"[\s\S]*showApplicationUpdateNotification\(version\)[\s\S]*1200/,
  )
})

test("렌더러 종료와 장기 무응답 상태를 자동 복구한다", () => {
  assert.match(
    electronMain,
    /webContents\.on\("render-process-gone"[\s\S]*recoverPrimaryRenderer/,
  )
  assert.match(
    electronMain,
    /window\.on\("unresponsive"[\s\S]*primaryRendererUnresponsiveTimeoutMs/,
  )
  assert.match(electronMain, /window\.webContents\.reload\(\)/)
  assert.doesNotMatch(electronMain, /\.isSkipTaskbar\(\)/)
  assert.match(
    electronMain,
    /primaryWindowSkippedFromTaskbar[\s\S]*window\.setSkipTaskbar\(primaryWindowSkippedFromTaskbar\)/,
  )
})

test("트레이 진입은 보조 창을 종료하고 복귀는 메인 창을 한 번만 표시한다", () => {
  const focusStart = electronMain.indexOf("function focusPrimaryWindow()")
  const focusEnd = electronMain.indexOf("function focusPrimaryWindowAfterTrayMenu()", focusStart)
  const focusSource = electronMain.slice(focusStart, focusEnd)
  const restoreStart = focusSource.indexOf("if (restoringFromTray) {")
  const restoreEnd = focusSource.indexOf("\n  if (!window.isVisible())", restoreStart)
  const restoreSource = focusSource.slice(restoreStart, restoreEnd)

  assert.match(
    electronMain,
    /function closeInternalWindowsForTray\(\)[\s\S]*internalWindowsClosedForTray\.add\(window\)[\s\S]*window\.destroy\(\)/,
  )
  assert.match(
    electronMain,
    /function minimizePrimaryWindowToTray\(\)[\s\S]*closeInternalWindowsForTray\(\)[\s\S]*primaryWindow\.hide\(\)/,
  )
  assert.match(focusSource, /setEnabled\(true\)[\s\S]*setFocusable\(true\)[\s\S]*setIgnoreMouseEvents\(false\)/)
  assert.match(restoreSource, /window\.show\(\)[\s\S]*writeWindowDiagnostics\("트레이 복귀 직후 창 상태"\)[\s\S]*return/)
  assert.doesNotMatch(restoreSource, /window\.focus\(\)|webContents\.focus\(\)/)
  assert.equal(
    electronMain.match(/internalWindowsClosedForTray\.delete\(window\)/g)?.length,
    5,
  )
  assert.match(
    electronMain,
    /function focusPrimaryWindowAfterTrayMenu\(\)[\s\S]*trayMenuCloseDelayMs[\s\S]*label: "열기"[\s\S]*focusPrimaryWindowAfterTrayMenu/,
  )
})

test("일반 창 포커스는 지연 후 상태를 확인하고 한 번만 재시도한다", () => {
  const focusStart = electronMain.indexOf("function focusPrimaryWindow()")
  const focusEnd = electronMain.indexOf("function focusPrimaryWindowAfterTrayMenu()", focusStart)
  const focusSource = electronMain.slice(focusStart, focusEnd)

  assert.match(focusSource, /const applyFocus = \(\) =>[\s\S]*window\.focus\(\)[\s\S]*window\.webContents\.focus\(\)/)
  assert.match(
    focusSource,
    /primaryWindowFocusTimer = setTimeout\([\s\S]*applyFocus\(\)[\s\S]*if \(!window\.isFocused\(\)\) applyFocus\(\)[\s\S]*primaryWindowFocusRetryDelayMs/,
  )
  assert.doesNotMatch(focusSource, /screen-saver|moveTop\(\)/)
})

test("트레이 종료는 메인 창 표시 여부에 맞는 종료 선택창을 사용한다", () => {
  assert.match(
    electronMain,
    /label: "종료"[\s\S]*requestApplicationExitFromTray\(\)/,
  )
  assert.match(
    electronMain,
    /function requestApplicationExitFromTray\(\) \{[\s\S]*primaryWindow\.isVisible\(\)[\s\S]*nativeDialog: !mainWindowVisible/,
  )
  assert.match(
    electronMain,
    /if \(mainWindowVisible\) focusPrimaryWindow\(\)[\s\S]*nativeDialog: !mainWindowVisible/,
  )
  assert.match(
    electronMain,
    /if \(nativeDialog\) \{[\s\S]*dialog\.showMessageBox\(\{[\s\S]*result\.response === 0 \? "reset" : "keep"/,
  )
  assert.doesNotMatch(
    electronMain,
    /function requestApplicationExitConfirmation[\s\S]{0,300}focusPrimaryWindow\(\)/,
  )
})

test("종료 모달은 업데이트 레이어보다 위에서 입력을 받는다", () => {
  assert.match(modalView, /\.modal-backdrop \{[\s\S]*z-index: 600;[\s\S]*-webkit-app-region: no-drag;/)
  assert.match(
    updateModalView,
    /\.update-preview-overlay \{[\s\S]*z-index: 500;[\s\S]*pointer-events: none;/,
  )
  assert.match(updateModalView, /button \{[\s\S]*pointer-events: auto;/)
  assert.match(
    updateModalView,
    /\.update-preview-panel \{[\s\S]*color: #000;[\s\S]*background: #ff9d00;/,
  )
  assert.match(
    updateModalView,
    /\.update-progress > span \{[\s\S]*font-weight: 900;[\s\S]*-webkit-text-stroke: 2px #000;/,
  )
  assert.match(updateModalView, /\.update-progress-value \{[\s\S]*background: #000;/)
  assert.match(
    applicationView,
    /\{#if !closeModalVisible && \([\s\S]*applicationUpdateState\.phase === "downloading"/,
  )
})

test("고급 기능 안내 말풍선은 실제 화면 노출을 최대 세 번 기록한다", () => {
  assert.match(
    electronMain,
    /async function readCreatorPromptDismissed\(\)[\s\S]*displayCount[\s\S]*>= 3/,
  )
  assert.match(
    electronMain,
    /async function recordCreatorPromptDisplay\(\)[\s\S]*displayCount: displayCount \+ 1/,
  )
  assert.match(
    electronPreload,
    /recordCreatorPromptDisplay: \(\) => ipcRenderer\.invoke\("application:record-creator-prompt-display"\)/,
  )
  assert.match(
    applicationView,
    /\$: creatorPromptVisible =[\s\S]*pageVisible[\s\S]*recordCreatorPromptDisplay\(\)/,
  )
})

test("Esc는 최상위 모달과 문서 화면을 닫고 모든 보조 창에도 적용된다", () => {
  assert.match(
    applicationView,
    /function closeTopLayerWithEscape\(\)[\s\S]*closeModalVisible[\s\S]*networkReconnectModalVisible[\s\S]*radeonGlobalModalVisible[\s\S]*turboTermsModalVisible[\s\S]*turboKeyModalVisible[\s\S]*conflictModalVisible[\s\S]*optimizationModalVisible[\s\S]*creatorViewPhase === "open"/,
  )
  assert.match(
    applicationView,
    /event\.key === "Escape" && closeTopLayerWithEscape\(\)[\s\S]*event\.stopImmediatePropagation\(\)/,
  )
  assert.match(
    electronMain,
    /function closeWindowOnEscape\(window, rendererEvent = ""\)[\s\S]*input\.type !== "keyDown"[\s\S]*input\.key !== "Escape"[\s\S]*window\.close\(\)/,
  )
  assert.equal(electronMain.match(/closeWindowOnEscape\(window(?:, "[^"]+")?\)/g)?.length, 5)
})

test("키보드 입력 후 버튼에 포커스 외곽선을 표시하지 않는다", () => {
  assert.match(
    applicationStyles,
    /button:focus-visible \{\s*outline: none;\s*\}/,
  )
  assert.doesNotMatch(applicationStyles, /button:focus-visible \{\s*outline: 2px solid/)
})

test("초기 상태 조회와 무관하게 창을 먼저 만들고 8초 안에 표시한다", () => {
  const createWindowAt = electronMain.indexOf("createWindow()", electronMain.indexOf("async function startApplication"))
  const loadPathAt = electronMain.indexOf("loadMabinogiExecutablePath()", createWindowAt)

  assert.ok(createWindowAt >= 0)
  assert.ok(loadPathAt > createWindowAt)
  assert.match(
    electronMain,
    /async function startApplication\(\)[\s\S]*configureApplicationUpdater\(\)[\s\S]*ensureApplicationTray\(\)[\s\S]*createWindow\(\)/,
  )
  assert.doesNotMatch(electronMain, /if \(startupTrayLaunch\) ensureApplicationTray\(\)/)
  assert.match(electronMain, /const primaryWindowRevealTimeoutMs = 8_000/)
  assert.match(
    electronMain,
    /primaryWindowRevealWatchdogTimer = setTimeout\([\s\S]*beginPrimaryWindowReveal\(\)[\s\S]*primaryWindowRevealTimeoutMs/,
  )
  assert.doesNotMatch(
    applicationView,
    /then\(launchContext => loadAll\(\)\.finally/,
  )
})

test("렌더러 생성 오류 18은 샌드박스 호환 모드로 한 번만 재실행한다", () => {
  assert.match(
    electronBootstrap,
    /process\.argv\.includes\("--sandbox-fallback"\)[\s\S]*app\.commandLine\.appendSwitch\("no-sandbox"\)/,
  )
  assert.match(
    electronMain,
    /function relaunchWithSandboxCompatibility\(details\)[\s\S]*details\?\.reason !== "launch-failed"[\s\S]*details\?\.exitCode !== 18[\s\S]*app\.relaunch\(\{ args \}\)[\s\S]*app\.exit\(0\)/,
  )
  assert.match(
    electronMain,
    /webContents\.on\("render-process-gone"[\s\S]*relaunchWithSandboxCompatibility\(details\)/,
  )
})

test("시작 이미지와 음악이 실패하거나 지연되어도 시작 애니메이션을 진행한다", () => {
  assert.match(gameWave, /playbackFallbackTimer = window\.setTimeout\([\s\S]*startAnimation\(\)[\s\S]*1500/)
  assert.match(
    gameWave,
    /image\.onerror = error => \{[\s\S]*imageReady = true[\s\S]*playStartup\(\)/,
  )
})

test("초기 부스트 상태가 확인되기 전에는 일시정지 배경으로 전환하지 않는다", () => {
  assert.match(
    applicationView,
    /function frameBoostStatusText\(\) \{[\s\S]*services\.affinity\.loading \|\| services\.memory\.loading[\s\S]*return "부스트 대기중"/,
  )
  assert.match(
    applicationView,
    /function syncDisplayedBoostStatus\(\)[\s\S]*nextPaused !== visualPaused[\s\S]*gameWave\?\.setPaused\(nextPaused\)/,
  )
})

test("부스트 중단은 일부 실행 상태도 원상복구하고 중단 상태를 단계별 표시한다", () => {
  const activeStateCheck = /!\(services\.affinity\.data\?\.running \|\| services\.memory\.data\?\.running\)/g
  assert.equal(applicationView.match(activeStateCheck)?.length, 2)
  assert.match(applicationView, /"부스트 중단중"/)
  assert.match(applicationView, /"부스트 적용 중단됨"/)
  assert.match(
    electronMain,
    /enabled[\s\S]*\? setAffinityEnabled\(\{ enabled: true, includeNic \}\)[\s\S]*: resetAllAffinities\(\)/,
  )
})

test("강제 간소화 미적용 상태는 빠른 런타임 조회마다 직접 확인한다", () => {
  assert.match(
    electronMain,
    /const cachedCharacterSimplification = fresh[\s\S]*const characterSimplification = cachedCharacterSimplification\?\.applied[\s\S]*: await getCharacterSimplificationStatus\(\)/,
  )
  assert.match(
    electronMain,
    /return \{[\s\S]*characterSimplification,[\s\S]*dxvk: dxvkRuntimeStatus/,
  )
})

test("helper 상태 파일 잠금 실패를 복구하고 중복 helper 실행을 막는다", () => {
  assert.match(electronMain, /import \{ writeJsonAtomic \} from "\.\.\/src\/atomic-json\.mjs"/)
  assert.match(
    electronMain,
    /async function acquireHelperLock\(statusPath,[\s\S]*openFile\(lockPath, "wx"\)[\s\S]*statusConfirmsHelper[\s\S]*existingStatus\?\.helperPid === existing\?\.pid[\s\S]*recentlyCreated \|\| statusConfirmsHelper/,
  )
  assert.match(
    electronMain,
    /Affinity 상태 기록 실패, 다음 주기에 다시 시도합니다/,
  )
  assert.match(
    electronMain,
    /메모리 상태 기록 실패, 다음 주기에 다시 시도합니다/,
  )
})

test("진단 로그는 최근 Windows 블루스크린과 비정상 종료 이벤트를 포함한다", () => {
  assert.match(
    electronMain,
    /async function readRecentWindowsFailureEvents\(\)[\s\S]*Get-WinEvent[\s\S]*Id = 41, 1001, 6008/,
  )
  assert.match(
    electronMain,
    /diagnosticResult\(\(\) => readRecentWindowsFailureEvents\(\)\)[\s\S]*recentWindowsFailures/,
  )
})

test("메모리 helper는 부모 앱 종료를 감지하고 기존 고아 helper를 정리한다", () => {
  assert.match(
    electronMain,
    /async function runMemoryHelper\(\)[\s\S]*parentPid = Number\(argumentValue\("parent-pid"\)\)[\s\S]*ownerPid: parentPid[\s\S]*while \(!stopping && isProcessRunning\(parentPid\)\)/,
  )
  assert.match(
    electronMain,
    /"--memory-helper",[\s\S]*`--parent-pid=\$\{process\.pid\}`/,
  )
  assert.match(
    electronMain,
    /orphanControlPath[\s\S]*reason: "orphan-recovery"[\s\S]*이전 helper가 종료되지 않았습니다/,
  )
})

test("부트스트랩이 시작 오류를 파일에 기록한다", () => {
  assert.equal(packageInfo.main, "electron/bootstrap.mjs")
  assert.match(electronBootstrap, /startup\.log/)
  assert.match(electronBootstrap, /process\.on\("uncaughtException"/)
  assert.match(electronBootstrap, /await import\("\.\/main\.mjs"\)/)
})

test("마비노기 운영정책 링크는 허용된 주소만 시스템 브라우저로 연다", () => {
  const policyUrlAt = applicationView.indexOf("const operationPolicyUrl")
  const termsParseAt = applicationView.indexOf("const turboTermsBlocks")

  assert.ok(policyUrlAt >= 0)
  assert.ok(termsParseAt > policyUrlAt)
  assert.match(
    turboKeyTermsDocument,
    /\[마비노기 '권장하지 않는 플레이 방식 안내'\]\(https:\/\/mabinogi\.nexon\.com\/page\/archive\/guide_view\.asp\?id=4889849&num=7&playtarget=1\)/,
  )
  assert.match(applicationView, /import turboKeyTermsMarkdown from "\.\.\/TURBO_KEY_TERMS\.md\?raw"/)
  assert.match(applicationView, /parseIntroduceMarkdown\(turboKeyTermsMarkdown\)/)
  assert.match(applicationView, /linkMatch\?\.\[2\] === operationPolicyUrl/)
  assert.match(applicationView, /window\.nogirem\.openOperationPolicy\(\)/)
  assert.match(electronPreload, /application:open-operation-policy/)
  assert.match(
    electronMain,
    /ipcMain\.handle\("application:open-operation-policy"[\s\S]*shell\.openExternal\(operationPolicyUrl\)/,
  )
})

test("버그 리포트 제출 버튼은 고정된 Google Forms 주소를 연다", () => {
  assert.match(applicationView, /window\.nogirem\.openBugReportForm\(\)/)
  assert.match(applicationView, />\s*제출하기\s*</)
  assert.match(electronPreload, /application:open-bug-report-form/)
  assert.match(
    electronMain,
    /const bugReportFormUrl = "https:\/\/docs\.google\.com\/forms\/d\/e\/1FAIpQLSfx6-QVqsxgUDKsYCMAyg7A51ZYBMrMa_17OGzzQF_gGOum1w\/viewform\?usp=publish-editor"/,
  )
  assert.match(
    electronMain,
    /ipcMain\.handle\("application:open-bug-report-form"[\s\S]*shell\.openExternal\(bugReportFormUrl\)/,
  )
})
