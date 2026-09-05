import assert from "node:assert/strict"
import test from "node:test"
import { advanceDownloadProgress } from "../src/update-progress.mjs"

test("업데이트 다운로드 진행률은 이전 값보다 감소하지 않는다", () => {
  assert.equal(advanceDownloadProgress(72.4, 65.1), 72.4)
  assert.equal(advanceDownloadProgress(72.4, 81.3), 81.3)
})

test("다운로드 완료 이벤트 전에는 진행률을 100으로 표시하지 않는다", () => {
  assert.equal(advanceDownloadProgress(98.7, 100), 99.9)
  assert.equal(advanceDownloadProgress(100, 65), 99.9)
})

test("잘못된 진행률 이벤트가 현재 표시를 초기화하지 않는다", () => {
  assert.equal(advanceDownloadProgress(54.2, undefined), 54.2)
  assert.equal(advanceDownloadProgress(54.2, Number.NaN), 54.2)
})
