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

test("트레이 복귀는 메뉴 종료 후 일반 포커스를 적용하고 한 번만 재시도한다", () => {
  const focusStart = electronMain.indexOf("function focusPrimaryWindow()")
  const focusEnd = electronMain.indexOf("function focusPrimaryWindowAfterTrayMenu()", focusStart)
  const focusSource = electronMain.slice(focusStart, focusEnd)

  assert.match(
    electronMain,
    /function hideInternalWindowsForTray\(\)[\s\S]*setIgnoreMouseEvents\(true\)[\s\S]*setAlwaysOnTop\(false\)[\s\S]*window\.hide\(\)/,
  )
  assert.match(
    electronMain,
    /function minimizePrimaryWindowToTray\(\)[\s\S]*hideInternalWindowsForTray\(\)[\s\S]*primaryWindow\.hide\(\)/,
  )
  assert.ok(focusSource.indexOf("restore()") < focusSource.indexOf("show()"))
  assert.match(focusSource, /const applyFocus = allowRetry =>[\s\S]*window\.focus\(\)[\s\S]*window\.webContents\.focus\(\)/)
  assert.match(focusSource, /primaryWindowFocusTimer = setTimeout\([\s\S]*applyFocus\(true\)/)
  assert.match(focusSource, /!window\.isFocused\(\)[\s\S]*primaryWindowFocusRetryDelayMs/)
  assert.doesNotMatch(focusSource, /screen-saver|moveTop\(\)/)
  assert.match(
    electronMain,
    /function focusPrimaryWindowAfterTrayMenu\(\)[\s\S]*trayMenuCloseDelayMs[\s\S]*label: "열기"[\s\S]*focusPrimaryWindowAfterTrayMenu/,
  )
  assert.equal(
    electronMain.match(/Window\.setIgnoreMouseEvents\(false\)/g)?.length,
    4,
  )
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
    applicationView,
    /\{#if !closeModalVisible && \([\s\S]*applicationUpdateState\.phase === "downloading"/,
  )
})

test("초기 상태 조회와 무관하게 창을 먼저 만들고 8초 안에 표시한다", () => {
  const createWindowAt = electronMain.indexOf("createWindow()", electronMain.indexOf("async function startApplication"))
  const loadPathAt = electronMain.indexOf("loadMabinogiExecutablePath()", createWindowAt)

  assert.ok(createWindowAt >= 0)
  assert.ok(loadPathAt > createWindowAt)
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

test("부스트 구성 요소가 일부만 실행됐으면 누를 때 나머지를 다시 시작한다", () => {
  const partialStateCheck = /!\(services\.affinity\.data\?\.running && services\.memory\.data\?\.running\)/g
  assert.equal(applicationView.match(partialStateCheck)?.length, 3)
  assert.doesNotMatch(
    applicationView,
    /!\(services\.affinity\.data\?\.running \|\| services\.memory\.data\?\.running\)/,
  )
})

test("Affinity 상태가 오래됐어도 강제 간소화 상태를 직접 확인한다", () => {
  assert.match(
    electronMain,
    /const characterSimplification = fresh && status\?\.characterSimplification[\s\S]*: await getCharacterSimplificationStatus\(\)/,
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
    /async function acquireHelperLock\(statusPath\)[\s\S]*openFile\(lockPath, "wx"\)[\s\S]*isProcessRunning\(existing\?\.pid\)/,
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
