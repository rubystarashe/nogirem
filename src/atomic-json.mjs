import { randomUUID } from "node:crypto"
import { mkdir, rename, unlink, writeFile } from "node:fs/promises"
import { dirname } from "node:path"
import { setTimeout as delay } from "node:timers/promises"

const retryableReplacementCodes = new Set([
  "EACCES",
  "EBUSY",
  "EEXIST",
  "ENOTEMPTY",
  "EPERM",
])
const replacementRetryDelaysMs = [15, 30, 60, 120, 240]

function isRetryableReplacementError(error) {
  return retryableReplacementCodes.has(error?.code)
}

async function retryReplacement(operation, wait, delays = replacementRetryDelaysMs) {
  let lastError
  for (let attempt = 0; attempt <= delays.length; attempt += 1) {
    try {
      return await operation()
    } catch (error) {
      if (!isRetryableReplacementError(error) || attempt === delays.length) throw error
      lastError = error
      await wait(delays[attempt])
    }
  }
  throw lastError
}

export async function writeJsonAtomic(path, value, operations = {}) {
  const write = operations.write ?? writeFile
  const move = operations.move ?? rename
  const remove = operations.remove ?? unlink
  const wait = operations.wait ?? delay
  const temporaryPath = `${path}.${process.pid}.${randomUUID()}.tmp`
  let moved = false

  await mkdir(dirname(path), { recursive: true })
  await write(temporaryPath, JSON.stringify(value), "utf8")
  try {
    try {
      await retryReplacement(() => move(temporaryPath, path), wait)
      moved = true
      return
    } catch (error) {
      if (!isRetryableReplacementError(error)) throw error
    }

    await retryReplacement(async () => {
      try {
        await remove(path)
      } catch (error) {
        if (error?.code !== "ENOENT") throw error
      }
    }, wait)
    await retryReplacement(() => move(temporaryPath, path), wait)
    moved = true
  } finally {
    if (!moved) await remove(temporaryPath).catch(() => {})
  }
}
