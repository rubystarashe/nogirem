import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"

const [applicationView, applicationStyles, serviceMain, servicePreload] = await Promise.all([
  readFile(new URL("../desktop/src/ui.rs", import.meta.url), "utf8"),
  readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
  readFile(new URL("../service/main.mjs", import.meta.url), "utf8"),
  readFile(new URL("../service/preload.js", import.meta.url), "utf8"),
])

test("고급 기능에 스킬 쿨타임 확인용 오버레이 인터페이스를 표시한다", () => {

  assert.match(applicationStyles, /\.overlay-tool-example/)
})

test("오버레이 인터페이스는 아직 다운로드나 실행 IPC를 연결하지 않는다", () => {
  assert.doesNotMatch(serviceMain, /overlay:(?:download|install|start|stop)/)
  assert.doesNotMatch(servicePreload, /(?:download|install|start|stop)Overlay/)
})
