import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import {
  defaultTurboKeyCodes,
  defaultTurboKeyIntervalMs,
  normalizeTurboKeyCodes,
  normalizeTurboKeyIntervalMs,
  turboKeyIntervalOptions,
} from "../src/turbo-key-settings.mjs"

test("터보 키 기본값은 선택된 키가 없는 상태다", () => {
  assert.deepEqual(defaultTurboKeyCodes, [])
})

test("터보 키 선택값은 지원 키만 중복 없이 정규화한다", () => {
  assert.deepEqual(normalizeTurboKeyCodes([112, 49, 112, 16, -1, "65"]), [49, 112])
  assert.deepEqual(normalizeTurboKeyCodes(null), defaultTurboKeyCodes)
})

test("터보 키 입력 간격은 지정된 선택지와 1ms 기본값만 사용한다", () => {
  assert.deepEqual(turboKeyIntervalOptions, [1, 3, 5, 10, 20, 30])
  assert.equal(defaultTurboKeyIntervalMs, 1)
  assert.equal(normalizeTurboKeyIntervalMs(3), 3)
  assert.equal(normalizeTurboKeyIntervalMs(2), 1)
})

test("터보 키 helper는 패키지에 포함되고 asar 밖에 배치된다", async () => {
  const packageInfo = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
  )
  const helperPath = "native/turbo-key/bin/turbo-key-helper.exe"

  assert.equal(packageInfo.scripts["native:turbo-key"], "node scripts/build-turbo-key.mjs")
  assert.equal(packageInfo.build.files.includes(helperPath), true)
  assert.equal(packageInfo.build.asarUnpack.includes(helperPath), true)
})

test("Windows 패키징 전에 터보 키 release 빌드를 실행한다", async () => {
  const script = await readFile(
    new URL("../scripts/package-win.mjs", import.meta.url),
    "utf8",
  )

  assert.match(script, /run\("npm", \["run", "native:turbo-key"\]\)/)
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
