import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"

const root = new URL("../", import.meta.url)

test("Alt Enter 방지는 마비노기 포그라운드의 Enter 입력만 소비한다", async () => {
  const source = await readFile(
    new URL("native/input-guard-helper/main.cpp", root),
    "utf8",
  )

  assert.match(
    source,
    /class AltEnterGuard[\s\S]*std::thread\([\s\S]*SetWindowsHookExW\([\s\S]*WH_KEYBOARD_LL[\s\S]*GetMessageW\(&message/,
  )
  assert.match(source, /event->vkCode == VK_RETURN/)
  assert.match(source, /LLKHF_ALTDOWN/)
  assert.match(
    source,
    /keyDown[\s\S]*altPressed[\s\S]*isTargetGameForeground[\s\S]*blockingEnter_ = true;[\s\S]*return 1;/,
  )
  assert.match(
    source,
    /keyUp && guard->blockingEnter_[\s\S]*blockingEnter_ = false;[\s\S]*return 1;/,
  )
  assert.match(source, /return CallNextHookEx\(nullptr, code, message, parameter\);/)
  assert.match(source, /WaitForSingleObject\(process, 0\) == WAIT_TIMEOUT/)
})

test("게임 포커스 중에만 접근성 커서 크기를 바꾸고 원래 값으로 복원한다", async () => {
  const source = await readFile(
    new URL("native/input-guard-helper/main.cpp", root),
    "utf8",
  )

  assert.match(source, /class CursorScaleGuard/)
  assert.match(
    source,
    /gameForeground = foregroundGame_\.matches\(\)[\s\S]*scalePercent_ != 100 && gameForeground/,
  )
  assert.match(source, /RegGetValueW\([\s\S]*L"CursorBaseSize"/)
  assert.match(source, /setCursorBaseSizeAction = 0x2029/)
  assert.match(
    source,
    /SystemParametersInfoW\([\s\S]*setCursorBaseSizeAction[\s\S]*SPIF_UPDATEINIFILE/,
  )
  assert.doesNotMatch(source, /SPIF_SENDCHANGE/)
  assert.match(source, /readCursorBaseSize\(\) != value/)
  assert.match(source, /MulDiv\(static_cast<int>\(originalBaseSize_\), scalePercent_, 100\)/)
  assert.doesNotMatch(source, /SPI_SETCURSORS/)
  assert.match(source, /originalCursorBaseSize/)
  assert.match(
    source,
    /if \(options\.restoreCursorBaseSize > 0\)[\s\S]*setCursorBaseSize/,
  )
  assert.match(
    source,
    /if \(options\.restoreOnly\) \{\s*ReleaseMutex\(mutex\)/,
  )
  assert.match(
    source,
    /changed_ = foreground != lastWindow_ \|\| pid != lastPid_[\s\S]*if \(!changed_\) return lastMatch_/,
  )
  assert.match(source, /CreateToolhelp32Snapshot\(TH32CS_SNAPPROCESS, 0\)/)
  assert.doesNotMatch(source, /PeekMessageW\(&message,[\s\S]*PM_REMOVE/)
  assert.match(source, /foregroundPid/)
  assert.match(source, /foregroundProcessName/)
  assert.match(source, /cursorScalePercent >= 75[\s\S]*cursorScalePercent <= 800/)
  assert.match(source, /std::clamp\([\s\S]*scalePercent_ \+ steps \* 25[\s\S]*800/)
  assert.match(source, /MulDiv\([\s\S]*scalePercent_[\s\S]*256/)
  assert.match(source, /SetWindowsHookExW\([\s\S]*WH_MOUSE_LL/)
  assert.match(source, /std::thread\([\s\S]*GetMessageW\(&message/)
  assert.match(source, /message != WM_MOUSEWHEEL/)
  assert.match(source, /VK_CONTROL/)
  assert.match(source, /VK_MENU/)
  assert.match(source, /foregroundPid != guard->foregroundGamePid_\.load/)
  assert.match(source, /pendingWheelSteps_\.fetch_add/)
  assert.match(source, /takeWheelSteps\(\)[\s\S]*pendingWheelSteps_\.exchange/)
  assert.match(source, /cursorScaleGuard\.adjustScale\(wheelSteps\)/)
  assert.match(source, /return 1;/)
  assert.match(
    source,
    /foregroundPath\.empty\(\)[\s\S]*processImageName\(pid\)[\s\S]*gameFileName_/,
  )
})

test("마비노기 입력 기능 설정은 앱 수명주기와 고급 기능 UI에 연결된다", async () => {
  const [main, preload, app, packageInfo, packageScript] = await Promise.all([
    readFile(new URL("service/main.mjs", root), "utf8"),
    readFile(new URL("service/preload.js", root), "utf8"),
    readFile(new URL("desktop/src/ui.rs", root), "utf8"),
    readFile(new URL("package.json", root), "utf8").then(JSON.parse),
    readFile(new URL("scripts/package-dioxus.mjs", root), "utf8"),
  ])

  assert.match(main, /async function launchInputGuardHelper\([^)]*\)/)
  assert.match(main, /async function stopInputGuardHelper\(\)/)
  assert.match(main, /async function ensureInputGuardStarted\(\)/)
  assert.match(main, /application:get-input-guard-setting/)
  assert.match(main, /application:set-input-guard-setting/)
  assert.match(main, /Array\.from\(\{ length: 30 \}, \(_, index\) => 75 \+ index \* 25\)/)
  assert.match(main, /inputGuardCursorWheelModifiers = new Set\(\["disabled", "control", "alt"\]\)/)
  assert.match(main, /`--alt-enter-enabled=\$\{setting\.enabled \? 1 : 0\}`/)
  assert.match(main, /`--cursor-scale-percent=\$\{setting\.cursorScalePercent\}`/)
  assert.match(main, /`--cursor-wheel-modifier=\$\{setting\.cursorWheelModifier\}`/)
  assert.match(main, /"--restore-only=1"/)
  assert.match(main, /function inputGuardCursorRestoreSize\(status\)/)
  assert.match(main, /`--restore-cursor-base-size=\$\{restoreCursorBaseSize\}`/)
  assert.match(main, /ensureInputGuardStarted/)
  assert.match(main, /stopInputGuardHelper\(\)/)
  assert.match(preload, /getInputGuardSetting/)
  assert.match(preload, /setInputGuardSetting/)

  assert.equal(
    packageInfo.scripts["native:input-guard"],
    "node scripts/build-input-guard-helper.mjs",
  )


  assert.match(packageScript, /['"]input-guard-helper['"]/)
})
