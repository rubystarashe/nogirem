export const reportResponseSourceUrl = (
  "https://raw.githubusercontent.com/rubystarashe/nogirem/master/REPORT.json"
)
export const maximumReportResponseBytes = 1024 * 1024
export const reportResponseRetentionDays = 7
export const reportIdPattern = (
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
)

export function normalizeReportResponseDocument(value) {
  const serialized = typeof value === "string" ? value : JSON.stringify(value)
  if (Buffer.byteLength(serialized, "utf8") > maximumReportResponseBytes) {
    throw new Error("버그 리포트 답변 문서가 허용 크기를 초과했습니다")
  }
  const document = typeof value === "string" ? JSON.parse(value) : value
  if (document?.schemaVersion !== 1 || !Array.isArray(document.responses)) {
    throw new Error("REPORT.json 형식이 올바르지 않습니다")
  }
  return {
    schemaVersion: 1,
    generatedAt: typeof document.generatedAt === "string"
      ? document.generatedAt
      : null,
    responses: document.responses.filter(response => (
      reportIdPattern.test(String(response?.reportId ?? ""))
      && typeof response?.responseId === "string"
      && response.responseId.length > 0
      && response.responseId.length <= 100
      && typeof response?.answeredAt === "string"
      && typeof response?.title === "string"
      && response.title.length > 0
      && response.title.length <= 200
      && typeof response?.message === "string"
      && response.message.length > 0
      && response.message.length <= 10_000
    )),
  }
}

export function selectOwnedReportResponses(
  document,
  ownedReportIds,
  acknowledgedResponseIds,
  now = Date.now(),
) {
  const owned = new Set(ownedReportIds)
  const acknowledged = new Set(acknowledgedResponseIds)
  return document.responses.filter(response => {
    const answeredAt = Date.parse(response.answeredAt)
    const expiresAt = response.expiresAt
      ? Date.parse(response.expiresAt)
      : Number.POSITIVE_INFINITY
    return owned.has(response.reportId)
      && Number.isFinite(answeredAt)
      && expiresAt > now
      && (response.forceDisplay === true || !acknowledged.has(response.responseId))
  })
}

export function pruneExpiredReportResponses(
  document,
  now = Date.now(),
  retentionDays = reportResponseRetentionDays,
) {
  const cutoff = now - retentionDays * 24 * 60 * 60 * 1000
  return {
    ...document,
    responses: document.responses.filter(response => {
      const answeredAt = Date.parse(response?.answeredAt)
      return Number.isFinite(answeredAt) && answeredAt >= cutoff
    }),
  }
}
