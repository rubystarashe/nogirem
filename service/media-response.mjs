import { createReadStream } from 'node:fs'
import { stat } from 'node:fs/promises'
import { Readable } from 'node:stream'

export const maximumMediaChunkBytes = 4 * 1024 * 1024

// IPC carries one bounded media range, never an entire recording in a JSON frame.
export async function localVideoResponse(filePath, request) {
  const { size: fileSize } = await stat(filePath)
  let start = 0, end = fileSize - 1
  let status = 200
  const range = request.headers.get('range')
  const invalid = () => new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${fileSize}` } })
  if (!fileSize) return new Response(null, { status: 200, headers: { 'Content-Length': '0', 'Content-Type': 'video/mp4' } })
  if (range) {
    const match = /^bytes=(\d*)-(\d*)$/i.exec(range.trim())
    if (!match || (!match[1] && !match[2])) return invalid()
    if (match[1]) {
      start = Number(match[1])
      if (match[2]) end = Number(match[2])
    } else {
      const length = Number(match[2])
      if (!Number.isSafeInteger(length) || length <= 0) return invalid()
      start = Math.max(0, fileSize - length)
    }
    if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0 || end < start || start >= fileSize) return invalid()
    end = Math.min(end, fileSize - 1)
    status = 206
  }
  if (request.method !== 'HEAD' && end - start + 1 > maximumMediaChunkBytes) {
    end = start + maximumMediaChunkBytes - 1
    status = 206
  }
  const headers = { 'Accept-Ranges': 'bytes', 'Content-Type': 'video/mp4', 'Content-Length': String(end - start + 1), 'Access-Control-Allow-Origin': 'http://dioxus.index.html' }
  if (status === 206) headers['Content-Range'] = `bytes ${start}-${end}/${fileSize}`
  return new Response(request.method === 'HEAD' ? null : Readable.toWeb(createReadStream(filePath, { start, end })), { status, headers })
}
