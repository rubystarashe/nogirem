import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"

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
