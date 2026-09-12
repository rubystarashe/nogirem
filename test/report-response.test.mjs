import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  normalizeReportResponseDocument,
  pruneExpiredReportResponses,
  selectOwnedReportResponses,
} from "../src/report-response.mjs"

const ownedReportId = "ad2dd07f-23a9-40c2-8340-a45fd5c068fb"
const otherReportId = "2290e251-d693-4edc-b738-34fd30cb52d4"

function response(overrides = {}) {
  return {
    reportId: ownedReportId,
    responseId: "response-1",
    answeredAt: "2026-09-12T12:00:00.000Z",
    expiresAt: "2026-10-12T12:00:00.000Z",
    title: "답변",
    message: "확인했습니다",
    ...overrides,
  }
}

test("자신이 추출한 진단 로그의 미확인 답변만 선택한다", () => {
  const document = normalizeReportResponseDocument({
    schemaVersion: 1,
    responses: [
      response(),
      response({ reportId: otherReportId, responseId: "response-2" }),
      response({ responseId: "response-3" }),
    ],
  })

  const selected = selectOwnedReportResponses(
    document,
    [ownedReportId],
    ["response-3"],
    Date.parse("2026-09-13T00:00:00.000Z"),
  )

  assert.deepEqual(selected.map(item => item.responseId), ["response-1"])
})

test("답변 시각이 7일 지난 항목을 정리한다", () => {
  const document = normalizeReportResponseDocument({
    schemaVersion: 1,
    responses: [
      response({ responseId: "old", answeredAt: "2026-09-01T00:00:00.000Z" }),
      response({ responseId: "recent", answeredAt: "2026-09-10T00:00:00.000Z" }),
    ],
  })

  const pruned = pruneExpiredReportResponses(
    document,
    Date.parse("2026-09-12T00:00:00.000Z"),
  )

  assert.deepEqual(pruned.responses.map(item => item.responseId), ["recent"])
})

test("REPORT 조회와 모달이 시작 및 업데이트 확인에 연결된다", async () => {
  const [mainSource, preloadSource, appSource, packageInfo, reportDocument] = await Promise.all([
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
    readFile(new URL("../package.json", import.meta.url), "utf8"),
    readFile(new URL("../REPORT.json", import.meta.url), "utf8"),
  ])

  assert.match(mainSource, /"If-None-Match"/)
  assert.match(mainSource, /response\.status === 304/)
  assert.match(mainSource, /applicationReportResponseNotifiedIds/)
  assert.match(mainSource, /void checkApplicationReportResponses\(\)/)
  assert.match(mainSource, /application:get-report-responses/)
  assert.match(mainSource, /application:report-responses-available/)
  assert.match(preloadSource, /getReportResponses/)
  assert.match(preloadSource, /onReportResponsesAvailable/)
  assert.match(appSource, /report-response-content/)
  assert.match(appSource, /리포트 고유값/)
  assert.match(packageInfo, /"REPORT\.json"/)
  assert.equal(normalizeReportResponseDocument(reportDocument).responses.length, 0)
})
