import assert from "node:assert/strict"
import test from "node:test"
import { createGraphicsManager } from "../src/graphics.mjs"

function nvidiaStatus(overrides = {}) {
  return {
    supported: true,
    nvidia: true,
    gpus: [{ name: "NVIDIA 테스트 GPU" }],
    goals: {
      verticalSyncOff: true,
      maxFrameRate400: true,
      threadedOptimizationOn: true,
      preferMaximumPerformance: true,
      ultraLowLatency: true,
      allMet: true,
    },
    ...overrides,
  }
}

function radeonStatus(overrides = {}) {
  return {
    supported: true,
    detected: true,
    vendor: "amd",
    vendorLabel: "AMD Radeon",
    gpus: [{ name: "AMD 테스트 GPU" }],
    goalsList: [],
    allMet: true,
    ...overrides,
  }
}

test("NVIDIA가 감지되면 Radeon 조회 없이 NVIDIA 계약을 반환한다", async () => {
  let radeonChecks = 0
  const manager = createGraphicsManager({
    checkNvidia: async () => nvidiaStatus(),
    applyNvidia: async () => nvidiaStatus(),
    checkRadeon: async () => {
      radeonChecks += 1
      return radeonStatus()
    },
    applyRadeon: async () => radeonStatus(),
  })

  const result = await manager.check("Client.exe")

  assert.equal(result.vendor, "nvidia")
  assert.equal(result.allMet, true)
  assert.equal(result.goalsList.length, 5)
  assert.equal(radeonChecks, 0)
})

test("NVIDIA가 없으면 Radeon으로 전환한다", async () => {
  const manager = createGraphicsManager({
    checkNvidia: async () => nvidiaStatus({
      nvidia: false,
      gpus: [],
      reason: "NVIDIA 없음",
    }),
    applyNvidia: async () => {
      throw new Error("호출되면 안 됨")
    },
    checkRadeon: async () => radeonStatus(),
    applyRadeon: async () => radeonStatus({ allMet: false }),
  })

  assert.equal((await manager.check("Client.exe")).vendor, "amd")
  assert.equal((await manager.apply("Client.exe")).allMet, false)
})

test("지원 GPU가 없으면 적용하지 않고 원인을 반환한다", async () => {
  const manager = createGraphicsManager({
    checkNvidia: async () => nvidiaStatus({
      nvidia: false,
      gpus: [],
      reason: "NVIDIA 없음",
    }),
    applyNvidia: async () => null,
    checkRadeon: async () => radeonStatus({
      detected: false,
      vendor: null,
      gpus: [],
      reason: "Radeon 없음",
    }),
    applyRadeon: async () => null,
  })

  const result = await manager.check("Client.exe")

  assert.equal(result.detected, false)
  assert.match(result.reason, /NVIDIA 없음/)
  assert.match(result.reason, /Radeon 없음/)
  await assert.rejects(() => manager.apply("Client.exe"), /NVIDIA 없음/)
})
