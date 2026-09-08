import { readFile, unlink } from "node:fs/promises"

export async function readJsonOrDiscard(path, onCorrupt = null) {
  try {
    return JSON.parse(await readFile(path, "utf8"))
  } catch (error) {
    if (error?.code === "ENOENT") return null
    if (!(error instanceof SyntaxError)) throw error
    onCorrupt?.(error, path)
    await unlink(path).catch(unlinkError => {
      if (unlinkError?.code !== "ENOENT") throw unlinkError
    })
    return null
  }
}

export const readRuntimeStatusJson = readJsonOrDiscard
