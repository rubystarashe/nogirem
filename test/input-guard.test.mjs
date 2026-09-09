import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"

const root = new URL("../", import.meta.url)

test("Alt Enter 방지는 마비노기 포그라운드의 Enter 입력만 소비한다", async () => {
  const source = await readFile(
    new URL("native/input-guard-helper/main.cpp", root),
    "utf8",
  )

  assert.match(source, /SetWindowsHookExW\([\s\S]*WH_KEYBOARD_LL/)
  assert.match(source, /event->vkCode == VK_RETURN/)
  assert.match(source, /LLKHF_ALTDOWN/)
  assert.match(
    source,
    /keyDown[\s\S]*altPressed[\s\S]*isTargetGameForeground[\s\S]*blockingEnter_ = true;[\s\S]*return 1;/,
  )
  assert.match(
    source,
    /keyUp && active_->blockingEnter_[\s\S]*blockingEnter_ = false;[\s\S]*return 1;/,
  )
  assert.match(source, /return CallNextHookEx\(nullptr, code, message, parameter\);/)
  assert.match(source, /WaitForSingleObject\(process, 0\) == WAIT_TIMEOUT/)
})

test("Alt Enter 방지 설정은 앱 수명주기와 고급 기능 UI에 연결된다", async () => {
  const [main, preload, app, packageInfo, packageScript] = await Promise.all([
    readFile(new URL("electron/main.mjs", root), "utf8"),
    readFile(new URL("electron/preload.cjs", root), "utf8"),
    readFile(new URL("web/App.svelte", root), "utf8"),
    readFile(new URL("package.json", root), "utf8").then(JSON.parse),
    readFile(new URL("scripts/package-win.mjs", root), "utf8"),
  ])

  assert.match(main, /async function launchInputGuardHelper\(\)/)
  assert.match(main, /async function stopInputGuardHelper\(\)/)
  assert.match(main, /async function ensureInputGuardStarted\(\)/)
  assert.match(main, /application:get-input-guard-setting/)
  assert.match(main, /application:set-input-guard-setting/)
  assert.match(main, /ensureInputGuardStarted/)
  assert.match(main, /stopInputGuardHelper\(\)/)
  assert.match(preload, /getInputGuardSetting/)
  assert.match(preload, /setInputGuardSetting/)
  assert.match(
    app,
    /Alt\+Enter 방지[\s\S]*마비노기 플레이 중 전체 화면 전환 단축키 Alt\+Enter 입력을 차단합니다/,
  )
  assert.equal(
    packageInfo.scripts["native:input-guard"],
    "node scripts/build-input-guard-helper.mjs",
  )
  assert.ok(
    packageInfo.build.files.includes(
      "native/input-guard-helper/bin/input-guard-helper.exe",
    ),
  )
  assert.ok(
    packageInfo.build.asarUnpack.includes(
      "native/input-guard-helper/bin/input-guard-helper.exe",
    ),
  )
  assert.match(packageScript, /run\("npm", \["run", "native:input-guard"\]\)/)
})
