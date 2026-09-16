import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"

const [applicationView, applicationStyles, electronMain, electronPreload] = await Promise.all([
  readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
  readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
  readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
  readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
])

test("고급 기능에 스킬 쿨타임 확인용 오버레이 인터페이스를 표시한다", () => {
  assert.match(
    applicationView,
    /<h2>오버레이<\/h2>[\s\S]*게임 화면의 원하는 영역을 복제해 화면 중앙 근처에 표시합니다/,
  )
  assert.match(applicationView, /스킬 슬롯 지정[\s\S]*쿨타임을 보기 쉬운 위치에 표시/)
  assert.match(
    applicationView,
    /기능 모듈은 터보 키처럼 별도 다운로드 방식으로 제공될 예정입니다/,
  )
  assert.match(applicationView, /<button disabled aria-label="오버레이 모듈 준비 중">/)
  assert.match(applicationStyles, /\.overlay-tool-example/)
})

test("오버레이 인터페이스는 아직 다운로드나 실행 IPC를 연결하지 않는다", () => {
  assert.doesNotMatch(electronMain, /overlay:(?:download|install|start|stop)/)
  assert.doesNotMatch(electronPreload, /(?:download|install|start|stop)Overlay/)
})
