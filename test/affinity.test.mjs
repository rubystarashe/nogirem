import test from "node:test"
import assert from "node:assert/strict"
import { buildCpuHalfMasks } from "../src/affinity.mjs"

test("CPU 절반 마스크를 논리 CPU 수에 맞게 동적으로 계산한다", () => {
  assert.deepEqual(buildCpuHalfMasks(4), {
    half: 2,
    allMask: 0xfn,
    lowerHalfMask: 0x3n,
    upperHalfMask: 0xcn,
  })
  assert.deepEqual(buildCpuHalfMasks(8), {
    half: 4,
    allMask: 0xffn,
    lowerHalfMask: 0x0fn,
    upperHalfMask: 0xf0n,
  })
  assert.deepEqual(buildCpuHalfMasks(12), {
    half: 6,
    allMask: 0xfffn,
    lowerHalfMask: 0x03fn,
    upperHalfMask: 0xfc0n,
  })
  assert.deepEqual(buildCpuHalfMasks(16), {
    half: 8,
    allMask: 0xffffn,
    lowerHalfMask: 0x00ffn,
    upperHalfMask: 0xff00n,
  })
})

test("지원하지 않는 논리 CPU 수는 마스크를 만들지 않는다", () => {
  assert.throws(() => buildCpuHalfMasks(2), /Unsupported logical CPU count/)
  assert.throws(() => buildCpuHalfMasks(7), /Unsupported logical CPU count/)
  assert.throws(() => buildCpuHalfMasks(54), /Unsupported logical CPU count/)
})
