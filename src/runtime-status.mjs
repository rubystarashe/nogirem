import { readFile, unlink } from "node:fs/promises"
import { setTimeout as delay } from "node:timers/promises"

export async function readJsonOrDiscard(
  path,
  onCorrupt = null,
  { missingRetryDelaysMs = [] } = {},
) {
  for (let attempt = 0; attempt <= missingRetryDelaysMs.length; attempt += 1) {
    try {
      return JSON.parse(await readFile(path, "utf8"))
    } catch (error) {
      if (error?.code === "ENOENT") {
        if (attempt === missingRetryDelaysMs.length) return null
        await delay(missingRetryDelaysMs[attempt])
        continue
      }
      if (!(error instanceof SyntaxError)) throw error
      onCorrupt?.(error, path)
      await unlink(path).catch(unlinkError => {
        if (unlinkError?.code !== "ENOENT") throw unlinkError
      })
      return null
    }
  }
  return null
}

export const readRuntimeStatusJson = readJsonOrDiscard
