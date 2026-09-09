import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import {
  defaultTurboKeyCodes,
  defaultTurboKeyIgnoreInitialDelay,
  defaultTurboKeyIntervalMs,
  normalizeTurboKeyCodes,
  normalizeTurboKeyIgnoreInitialDelay,
  normalizeTurboKeyIntervalMs,
  turboKeyIntervalOptions,
} from "../src/turbo-key-settings.mjs"
import { turboKeyHelperAssetName } from "../src/turbo-key-installer.mjs"

test("터보 키 기본값은 선택된 키가 없는 상태다", () => {
  assert.deepEqual(defaultTurboKeyCodes, [])
})

test("터보 키 선택값은 지원 키만 중복 없이 정규화한다", () => {
  assert.deepEqual(normalizeTurboKeyCodes([112, 49, 112, 27, 16, -1, "65"]), [49, 112])
  assert.deepEqual(normalizeTurboKeyCodes(null), defaultTurboKeyCodes)
})

test("터보 키 입력 간격은 지정된 선택지와 1ms 기본값만 사용한다", () => {
  assert.deepEqual(turboKeyIntervalOptions, [1, 3, 5, 10, 20, 30])
  assert.equal(defaultTurboKeyIntervalMs, 1)
  assert.equal(normalizeTurboKeyIntervalMs(3), 3)
  assert.equal(normalizeTurboKeyIntervalMs(2), 1)
})

test("터보 키 최초 입력 지연 무시는 명시적인 true만 허용한다", () => {
  assert.equal(defaultTurboKeyIgnoreInitialDelay, false)
  assert.equal(normalizeTurboKeyIgnoreInitialDelay(true), true)
  assert.equal(normalizeTurboKeyIgnoreInitialDelay(false), false)
  assert.equal(normalizeTurboKeyIgnoreInitialDelay("true"), false)
})

test("터보 키 helper는 설치본에 포함되지 않는다", async () => {
  const packageInfo = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
  )
  const helperPath = "native/turbo-key/bin/turbo-key-helper.exe"

  assert.equal(packageInfo.scripts["native:turbo-key"], "node scripts/build-turbo-key.mjs")
  assert.equal(packageInfo.build.files.includes(helperPath), false)
  assert.equal(packageInfo.build.asarUnpack.includes(helperPath), false)
})

test("Windows 패키징은 터보 키를 별도 Release 자산으로 준비한다", async () => {
  const script = await readFile(
    new URL("../scripts/package-win.mjs", import.meta.url),
    "utf8",
  )

  assert.match(script, /run\("npm", \["run", "native:turbo-key"\]\)/)
  assert.match(script, /uploadTurboKeyHelper/)
  assert.match(script, /copyFile\(turboKeyHelperSource, turboKeyHelperOutput\)/)
  assert.match(turboKeyHelperAssetName, /^turbo-key-helper-win32-x64-v[\d.]+\.exe$/)
  const buildScript = await readFile(
    new URL("../scripts/build-turbo-key.mjs", import.meta.url),
    "utf8",
  )
  assert.match(buildScript, /"x86_64-pc-windows-msvc"/)
})

test("터보 키 helper는 게임 외 P-core 마스크를 시작 인자로 받는다", async () => {
  const electronMain = await readFile(
    new URL("../electron/main.mjs", import.meta.url),
    "utf8",
  )

  assert.match(electronMain, /const latencyMask = allocation\.alternateGameMask \|\| allocation\.backgroundMask/)
  assert.match(electronMain, /`--affinity-mask=0x\$\{latencyMask\.toString\(16\)\}`/)
  assert.match(electronMain, /`--interval-ms=\$\{normalizeTurboKeyIntervalMs\(intervalMs\)\}`/)
})

test("미설치 helper는 자동 실행하지 않고 제한된 다운로드 IPC만 제공한다", async () => {
  const [electronMain, electronPreload] = await Promise.all([
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
  ])

  assert.match(
    electronMain,
    /async function ensureTurboKeyStarted\(\)[\s\S]*if \(!installation\.installed\) return[\s\S]*launchTurboKeyHelper/,
  )
  assert.match(electronMain, /application:download-turbo-key-helper/)
  assert.match(electronMain, /application:remove-turbo-key-helper/)
  assert.match(
    electronMain,
    /async function downloadTurboKeyHelper\(\)[\s\S]*enabled: false[\s\S]*return getTurboKeySetting\(\)/,
  )
  assert.match(
    electronMain,
    /BrowserWindow\.fromWebContents\(event\.sender\) !== primaryWindow[\s\S]*downloadTurboKeyHelper\(\)/,
  )
  assert.match(electronPreload, /downloadTurboKeyHelper:[\s\S]*application:download-turbo-key-helper/)
  assert.match(electronPreload, /removeTurboKeyHelper:[\s\S]*application:remove-turbo-key-helper/)
})

test("터보 키를 사용하기 전에 설정 모달을 열고 사용 중일 때만 키 설정을 표시한다", async () => {
  const applicationView = await readFile(
    new URL("../web/App.svelte", import.meta.url),
    "utf8",
  )

  assert.match(
    applicationView,
    /async function toggleTurboKey\(\)[\s\S]*if \(!turboKeyEnabled\) \{[\s\S]*turboKeyEnableAfterSettings = true[\s\S]*turboKeyModalVisible = true[\s\S]*return/,
  )
  assert.match(
    applicationView,
    /enabled: turboKeyEnableAfterSettings \|\| turboKeyEnabled/,
  )
  assert.match(
    applicationView,
    /\{#if turboKeyEnabled\}[\s\S]*openTurboKeySettings[\s\S]*키 설정[\s\S]*\{\/if\}/,
  )
  assert.doesNotMatch(applicationView, /사용 안 함/)
  assert.match(applicationView, /turboKeyEnabled[\s\S]*"사용하기"/)
  assert.match(
    applicationView,
    /스킬 키를 미리 누르고 있어도 스킬이 사용될 수 있도록 합니다\. 기능을 사용하려면 다운로드가 필요합니다/,
  )
})

test("Esc는 터보 키에서 제외하고 키캡 hover 색상을 사용하지 않는다", async () => {
  const [applicationView, applicationStyles, helperSource] = await Promise.all([
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
    readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
    readFile(new URL("../native/turbo-key/src/main.rs", import.meta.url), "utf8"),
  ])

  assert.match(applicationView, /\{ label: "Esc", disabled: true \}/)
  assert.doesNotMatch(applicationStyles, /\.turbo-keycap:hover/)
  assert.match(helperSource, /is_excluded_key[\s\S]*VK_ESCAPE/)
})

test("터보 키 helper는 정밀 타이머와 우선 스케줄링을 사용한다", async () => {
  const helperSource = await readFile(
    new URL("../native/turbo-key/src/main.rs", import.meta.url),
    "utf8",
  )

  assert.match(helperSource, /CREATE_WAITABLE_TIMER_HIGH_RESOLUTION/)
  assert.match(helperSource, /SetPriorityClass\(GetCurrentProcess\(\), ABOVE_NORMAL_PRIORITY_CLASS\)/)
  assert.match(
    helperSource,
    /apply_current_thread_priority\(Some\(ideal_processor\), THREAD_PRIORITY_HIGHEST\)/,
  )
  assert.match(helperSource, /SetThreadIdealProcessor\(thread, processor\)/)
  assert.match(helperSource, /is_process_foreground\(foreground_pid\)/)
  assert.match(helperSource, /MsgWaitForMultipleObjectsEx/)
  assert.match(helperSource, /HEALTH_CHECK_INTERVAL_MS: u32 = 250/)
  assert.match(helperSource, /INITIAL_REPEAT_DELAY_MS: u64 = 250/)
  assert.doesNotMatch(helperSource, /SPI_GETKEYBOARDDELAY|SystemParametersInfoW/)
  assert.match(
    helperSource,
    /apply_current_thread_priority\(None, THREAD_PRIORITY_ABOVE_NORMAL\)/,
  )
  assert.doesNotMatch(helperSource, /thread::sleep\(Duration::from_millis\(5\)\)/)
})

test("터보 키 입력 지연 무시 설정은 UI부터 helper까지 전달된다", async () => {
  const [applicationView, electronMain, helperSource] = await Promise.all([
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../native/turbo-key/src/main.rs", import.meta.url), "utf8"),
  ])

  assert.match(applicationView, /키보드 입력 지연을 무시하고 즉시 입력/)
  assert.match(
    applicationView,
    /\{#if turboKeyInstalled && turboKeyEnabled\}[\s\S]*class="turbo-key-immediate-option"/,
  )
  assert.match(applicationView, /ignoreInitialDelay: !turboKeyIgnoreInitialDelay/)
  assert.match(electronMain, /--ignore-initial-delay=\$\{normalizeTurboKeyIgnoreInitialDelay/)
  assert.match(helperSource, /initial_repeat_delay: Duration/)
  assert.match(helperSource, /Instant::now\(\) \+ shared\.initial_repeat_delay/)
  assert.match(helperSource, /if ignore_initial_delay \{\s*0\s*\} else \{\s*INITIAL_REPEAT_DELAY_MS/)
})

test("터보 키 설치 무결성은 매초 다시 계산하지 않고 실행 전에 갱신한다", async () => {
  const electronMain = await readFile(
    new URL("../electron/main.mjs", import.meta.url),
    "utf8",
  )

  assert.match(electronMain, /let turboKeyInstallationCache = null/)
  assert.match(
    electronMain,
    /getCachedTurboKeyInstallation\(paths\.directory\)/,
  )
  assert.match(
    electronMain,
    /getCachedTurboKeyInstallation\([\s\S]*\{ refresh: true \}[\s\S]*\)/,
  )
})

test("개발 모드는 AppData 복사본 대신 로컬 빌드 helper를 직접 실행한다", async () => {
  const electronMain = await readFile(
    new URL("../electron/main.mjs", import.meta.url),
    "utf8",
  )

  assert.match(
    electronMain,
    /const localTurboKeyHelperPath = join\([\s\S]*"native"[\s\S]*"turbo-key-helper\.exe"/,
  )
  assert.match(
    electronMain,
    /turboKeyInstallationCache = app\.isPackaged[\s\S]*getTurboKeyHelperInstallation\(directory\)[\s\S]*getLocalTurboKeyHelper\(localTurboKeyHelperPath\)/,
  )
  assert.match(
    electronMain,
    /const executablePath = installation\.executablePath[\s\S]*spawn\(executablePath/,
  )
})

test("기존 동의를 유지한 채 구버전 터보 키 helper를 시작 시 자동 교체한다", async () => {
  const [electronMain, installerSource] = await Promise.all([
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../src/turbo-key-installer.mjs", import.meta.url), "utf8"),
  ])

  assert.match(installerSource, /turboKeyHelperVersion = "0\.1\.7"/)
  assert.match(
    installerSource,
    /const updateRequired = manifest\.helperVersion !== turboKeyHelperVersion/,
  )
  assert.match(
    electronMain,
    /async function updateTurboKeyHelperIfNeeded\(installation\)[\s\S]*installation\.acceptedAt[\s\S]*installTurboKeyHelper/,
  )
  assert.match(
    electronMain,
    /async function ensureTurboKeyStarted\(\)[\s\S]*updateTurboKeyHelperIfNeeded\(installation\)[\s\S]*if \(!settings\?\.enabled\) return/,
  )
})
