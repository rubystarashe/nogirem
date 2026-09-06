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
