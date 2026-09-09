import assert from "node:assert/strict"
import test from "node:test"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { readJsonOrDiscard, readRuntimeStatusJson } from "../src/runtime-status.mjs"

async function withTemporaryDirectory(run) {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-runtime-status-"))
  try {
    await run(directory)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

test("정상 런타임 상태 JSON을 읽는다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "status.json")
    await writeFile(path, JSON.stringify({ running: true }), "utf8")

    assert.deepEqual(await readRuntimeStatusJson(path), { running: true })
  })
})

test("원자 교체 중 잠시 사라진 영구 설정 파일을 다시 읽는다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "settings.json")
    const writing = new Promise((resolve, reject) => {
      setTimeout(() => {
        void writeFile(path, JSON.stringify({ maxDurationSeconds: 18000 }), "utf8")
          .then(resolve, reject)
      }, 10)
    })

    assert.deepEqual(
      await readJsonOrDiscard(path, null, {
        missingRetryDelaysMs: [5, 20],
      }),
      { maxDurationSeconds: 18000 },
    )
    await writing
  })
})

test("NUL 문자로 손상된 런타임 상태 파일을 폐기한다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "status.json")
    await writeFile(path, Buffer.alloc(588))
    const corruptions = []

    assert.equal(
      await readRuntimeStatusJson(path, error => corruptions.push(error)),
      null,
    )
    assert.equal(corruptions.length, 1)
    await assert.rejects(readFile(path), error => error?.code === "ENOENT")
  })
})

test("일부만 기록된 런타임 상태 파일도 폐기한다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "status.json")
    await writeFile(path, '{"running":', "utf8")

    assert.equal(await readRuntimeStatusJson(path), null)
    await assert.rejects(readFile(path), error => error?.code === "ENOENT")
  })
})

test("NUL 문자로 손상된 영구 설정도 기본값으로 복구할 수 있다", async () => {
  await withTemporaryDirectory(async directory => {
    const path = join(directory, "game-core-setting.json")
    await writeFile(path, Buffer.alloc(64))
    const corruptions = []

    assert.equal(
      await readJsonOrDiscard(path, error => corruptions.push(error)),
      null,
    )
    assert.equal(corruptions.length, 1)
    await assert.rejects(readFile(path), error => error?.code === "ENOENT")
  })
})
