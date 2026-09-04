import assert from "node:assert/strict"
import { dirname, resolve } from "node:path"
import test from "node:test"
import { fileURLToPath } from "node:url"
import { getLatestMuoStatus } from "../src/muo-status.mjs"

const root = dirname(dirname(fileURLToPath(import.meta.url)))

test("간소화 설정 MUO에서 DummyCharRenderModeFPS 값을 읽는다", async () => {
  const status = await getLatestMuoStatus(resolve(root, "assets"))

  assert.equal(status.attribute, "DummyCharRenderModeFPS")
  assert.equal(status.value, "-1")
  assert.equal(status.applied, true)
})
