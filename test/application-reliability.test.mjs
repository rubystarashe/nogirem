import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"

const electronMain = await readFile(
  new URL("../electron/main.mjs", import.meta.url),
  "utf8",
)

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
})
