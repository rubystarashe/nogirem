import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"

const installerSource = await readFile(
  new URL("../build/installer.nsh", import.meta.url),
  "utf8",
)
const packageInfo = JSON.parse(
  await readFile(new URL("../package.json", import.meta.url), "utf8"),
)

test("설치 완료 자동 실행은 시작 메뉴 바로가기 대신 설치 EXE를 사용한다", () => {
  assert.equal(packageInfo.build.nsis.runAfterFinish, true)
  assert.match(
    installerSource,
    /!macro customInstall\s+StrCpy \$launchLink "\$INSTDIR\\\$\{APP_EXECUTABLE_FILENAME\}"\s+!macroend/,
  )
})
