import { createHash } from "node:crypto"

export const applicationNoticeSourceUrl = (
  "https://raw.githubusercontent.com/rubystarashe/nogirem/master/NOTICE.md"
)
export const maximumApplicationNoticeBytes = 1024 * 1024

export function normalizeApplicationNotice(markdown) {
  const normalized = String(markdown ?? "").replace(/\r\n/g, "\n").trim()
  if (!normalized) return null
  if (Buffer.byteLength(normalized, "utf8") > maximumApplicationNoticeBytes) {
    throw new Error("공지사항 문서가 허용 크기를 초과했습니다")
  }
  return {
    id: createHash("sha256").update(normalized, "utf8").digest("hex"),
    markdown: normalized,
  }
}

export function shouldDisplayApplicationNotice(notice, dismissedId) {
  return Boolean(notice && notice.id !== dismissedId)
}
