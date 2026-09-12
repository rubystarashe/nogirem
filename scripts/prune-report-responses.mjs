import { readFile, writeFile } from "node:fs/promises"
import { resolve } from "node:path"
import {
  normalizeReportResponseDocument,
  pruneExpiredReportResponses,
  reportResponseRetentionDays,
} from "../src/report-response.mjs"

const reportPath = resolve(process.cwd(), "REPORT.json")
const retentionDays = Number(
  process.env.REPORT_RETENTION_DAYS ?? reportResponseRetentionDays,
)

if (!Number.isFinite(retentionDays) || retentionDays < 1) {
  throw new Error("REPORT_RETENTION_DAYS는 1 이상의 숫자여야 합니다")
}

const document = normalizeReportResponseDocument(
  await readFile(reportPath, "utf8"),
)
const pruned = pruneExpiredReportResponses(document, Date.now(), retentionDays)
const removedCount = document.responses.length - pruned.responses.length

if (removedCount > 0) {
  pruned.generatedAt = new Date().toISOString()
  await writeFile(reportPath, `${JSON.stringify(pruned, null, 2)}\n`, "utf8")
}

console.log(`REPORT 응답 ${removedCount}개 제거, ${pruned.responses.length}개 유지`)
