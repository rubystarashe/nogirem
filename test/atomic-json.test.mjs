import assert from "node:assert/strict"
import test from "node:test"
import { mkdtemp, readFile, rename, rm, unlink, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { writeJsonAtomic } from "../src/atomic-json.mjs"

async function withTemporaryDirectory(run) {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-atomic-json-"))
  try {
    await run(directory)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

function replacementError(code = "EPERM") {
  return Object.assign(new Error("파일 교체 실패"), { code })
}

test("Windows 파일 잠금이 잠시 발생하면 JSON 교체를 재시도한다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "status.json")
    let moveCount = 0

    await writeJsonAtomic(path, { running: true }, {
      move: async (source, destination) => {
        moveCount += 1
        if (moveCount < 3) throw replacementError()
        await rename(source, destination)
      },
      wait: async () => {},
    })

    assert.equal(moveCount, 3)
    assert.deepEqual(JSON.parse(await readFile(path, "utf8")), { running: true })
  })
})

test("재시도 후에도 교체가 막히면 기존 파일을 삭제하고 대체한다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "status.json")
    await writeFile(path, JSON.stringify({ running: false }), "utf8")
    let removed = false

    await writeJsonAtomic(path, { running: true }, {
      move: async (source, destination) => {
        if (!removed) throw replacementError()
        await rename(source, destination)
      },
      remove: async target => {
        if (target === path) removed = true
        await unlink(target)
      },
      wait: async () => {},
    })

    assert.equal(removed, true)
    assert.deepEqual(JSON.parse(await readFile(path, "utf8")), { running: true })
  })
})
