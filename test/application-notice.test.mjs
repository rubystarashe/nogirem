import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"
import {
  normalizeApplicationNotice,
  shouldDisplayApplicationNotice,
} from "../src/application-notice.mjs"

const [mainSource, preloadSource, appSource, noticeModalSource, packageInfo, noticeMarkdown] = await Promise.all([
  readFile(new URL("../service/main.mjs", import.meta.url), "utf8"),
  readFile(new URL("../service/preload.js", import.meta.url), "utf8"),
  readFile(new URL("../desktop/src/ui.rs", import.meta.url), "utf8"),
  readFile(new URL("../desktop/src/ui.css", import.meta.url), "utf8"),
  readFile(new URL("../scripts/package-dioxus.mjs", import.meta.url), "utf8"),
  readFile(new URL("../NOTICE.md", import.meta.url), "utf8"),
])

test("공지 내용이 바뀔 때만 새 공지로 판정한다", () => {
  const notice = normalizeApplicationNotice("# 공지\n\n새 내용")
  const changedNotice = normalizeApplicationNotice("# 공지\n\n변경된 내용")

  assert.equal(notice.id.length, 64)
  assert.equal(shouldDisplayApplicationNotice(notice, null), true)
  assert.equal(shouldDisplayApplicationNotice(notice, notice.id), false)
  assert.equal(shouldDisplayApplicationNotice(changedNotice, notice.id), true)
})

test("공지 조회와 닫기 상태가 메인 창 IPC 및 모달에 연결된다", () => {
  assert.doesNotMatch(mainSource, /forceApplicationNoticePreview/)
  assert.match(mainSource, /fetch\(applicationNoticeSourceUrl,[\s\S]*cache: "no-store"/)
  assert.match(
    mainSource,
    /shouldDisplayApplicationNotice\(\s*notice,\s*dismissed\?\.id,\s*\)/,
  )
  assert.match(mainSource, /application:notice-available/)
  assert.match(mainSource, /application:get-notice/)
  assert.match(mainSource, /application:dismiss-notice/)
  assert.match(mainSource, /void checkApplicationNotice\(\)[\s\S]*if \(!app\.isPackaged\)/)
  assert.match(preloadSource, /getNotice:[\s\S]*application:get-notice/)
  assert.match(preloadSource, /onNoticeAvailable:[\s\S]*application:notice-available/)

  assert.match(packageInfo, /['"]NOTICE\.md['"]/)
})
