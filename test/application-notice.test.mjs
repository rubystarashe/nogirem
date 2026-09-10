import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"
import {
  normalizeApplicationNotice,
  shouldDisplayApplicationNotice,
} from "../src/application-notice.mjs"

const [mainSource, preloadSource, appSource, noticeModalSource, packageInfo, noticeMarkdown] = await Promise.all([
  readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
  readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
  readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
  readFile(new URL("../web/NoticeModal.svelte", import.meta.url), "utf8"),
  readFile(new URL("../package.json", import.meta.url), "utf8"),
  readFile(new URL("../NOTICE.md", import.meta.url), "utf8"),
])

test("공지 내용이 바뀔 때만 새 공지로 판정한다", () => {
  const notice = normalizeApplicationNotice("# 공지\n\n새 내용")

  assert.equal(notice.id.length, 64)
  assert.equal(shouldDisplayApplicationNotice(notice, null), true)
  assert.equal(shouldDisplayApplicationNotice(notice, notice.id), false)
  assert.equal(shouldDisplayApplicationNotice(notice, notice.id, true), true)
})

test("공지 문서에 염색 도우미 이미지와 HTTPS 설문 링크가 포함된다", () => {
  assert.match(
    noticeMarkdown,
    /!\[[^\]]+\]\(https:\/\/raw\.githubusercontent\.com\/rubystarashe\/nogirem\/master\/notice\/noticeimage\.png\)/,
  )
  assert.match(noticeMarkdown, /\[[^\]]+\]\(https:\/\/docs\.google\.com\/forms\//)
})

test("공지 조회와 닫기 상태가 메인 창 IPC 및 모달에 연결된다", () => {
  assert.match(mainSource, /const forceApplicationNoticePreview = true/)
  assert.match(mainSource, /application:notice-available/)
  assert.match(mainSource, /application:get-notice/)
  assert.match(mainSource, /application:dismiss-notice/)
  assert.match(mainSource, /void checkApplicationNotice\(\)[\s\S]*if \(!app\.isPackaged\)/)
  assert.match(preloadSource, /getNotice:[\s\S]*application:get-notice/)
  assert.match(preloadSource, /onNoticeAvailable:[\s\S]*application:notice-available/)
  assert.match(appSource, /parseApplicationNotice[\s\S]*applicationNoticeVisible = true/)
  assert.match(appSource, /<NoticeModal[\s\S]*application-notice-content[\s\S]*MarkdownBlocks/)
  assert.match(appSource, /application-notice-title[\s\S]*applicationNoticeTitle/)
  assert.match(appSource, /application-notice-confirm[\s\S]*확인/)
  assert.match(noticeModalSource, /align-items: center/)
  assert.match(noticeModalSource, /padding: 24px/)
  assert.match(noticeModalSource, /padding: 14px 16px/)
  assert.match(noticeModalSource, /background: #fff/)
  assert.match(noticeModalSource, /\.notice-content::-webkit-scrollbar[\s\S]*width: 3px/)
  assert.match(noticeModalSource, /\.notice-content::-webkit-scrollbar-thumb/)
  assert.match(noticeModalSource, /margin-right: -12px/)
  assert.match(noticeModalSource, /function smoothScroll\(event\)/)
  assert.match(noticeModalSource, /scrollElement\.scrollTop \+= distance \* 0\.18/)
  assert.match(noticeModalSource, /onwheel=\{smoothScroll\}/)
  assert.doesNotMatch(noticeModalSource, /class="notice-actions"/)
  assert.doesNotMatch(noticeModalSource, /<h2>\{title\}<\/h2>/)
  assert.doesNotMatch(noticeModalSource, /class="notice-close"/)
  assert.match(packageInfo, /"NOTICE\.md"/)
})
