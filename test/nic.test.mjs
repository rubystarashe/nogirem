import assert from "node:assert/strict"
import test from "node:test"
import {
  applyNicRssAffinity,
  buildNicRssAffinityPlan,
  getNicRssAffinityStatus,
  isNicRssAffinityOptimized,
  restoreNicRssAffinity,
} from "../src/nic.mjs"

const disabledStatus = {
  interfaceAlias: "이더넷",
  interfaceIndex: 14,
  interfaceDescription: "Realtek PCIe 5GbE Family Controller",
  adapterStatus: "Up",
  enabled: false,
  profile: null,
  baseProcessorGroup: null,
  baseProcessorNumber: null,
  maxProcessorGroup: null,
  maxProcessorNumber: null,
  maxProcessors: null,
  numberOfReceiveQueues: 4,
}

const optimizedStatus = {
  ...disabledStatus,
  enabled: true,
  profile: "ClosestStatic",
  baseProcessorGroup: 0,
  baseProcessorNumber: 0,
  maxProcessorGroup: 0,
  maxProcessorNumber: 7,
  maxProcessors: 8,
}

function createMemoryStateStore(initial = null) {
  let value = initial
  return {
    async load() {
      return value
    },
    async save(next) {
      value = structuredClone(next)
    },
    async remove() {
      value = null
    },
    read() {
      return value
    },
  }
}

test("16개 논리 CPU에서 NIC RSS 목표를 0~7로 계산한다", () => {
  const plan = buildNicRssAffinityPlan(disabledStatus, { logicalCpuCount: 16 })
  assert.equal(plan.baseProcessorNumber, 0)
  assert.equal(plan.maxProcessorNumber, 7)
  assert.equal(plan.maxProcessors, 8)
  assert.equal(plan.gameProcessorStart, 8)
  assert.equal(plan.gameProcessorEnd, 15)
  assert.equal(plan.profile, "ClosestStatic")
})

test("RSS가 꺼져 있으면 최적화되지 않은 상태로 판정한다", async () => {
  const result = await getNicRssAffinityStatus({
    runner: async () => disabledStatus,
    logicalCpuCount: 16,
  })
  assert.equal(result.optimized, false)
  assert.equal(result.rssEnabled, false)
  assert.equal(result.gameCpuOverlap, true)
})

test("RSS 범위가 0~7이면 최적화된 상태로 판정한다", () => {
  const plan = buildNicRssAffinityPlan(optimizedStatus, { logicalCpuCount: 16 })
  assert.equal(isNicRssAffinityOptimized(optimizedStatus, plan), true)
})

test("드라이런은 NIC RSS 설정과 원본 상태를 변경하지 않는다", async () => {
  const stateStore = createMemoryStateStore()
  const actions = []
  const result = await applyNicRssAffinity({
    applyChanges: false,
    runner: async action => {
      actions.push(action)
      return disabledStatus
    },
    stateStore,
    logicalCpuCount: 16,
  })
  assert.deepEqual(actions, ["status"])
  assert.equal(result.applied, false)
  assert.equal(stateStore.read(), null)
})

test("적용 함수는 원본을 저장하고 RSS를 0~7에 배치한다", async () => {
  const stateStore = createMemoryStateStore()
  const actions = []
  const result = await applyNicRssAffinity({
    applyChanges: true,
    runner: async (action, payload) => {
      actions.push(action)
      if (action === "status") return disabledStatus
      assert.equal(payload.baseProcessorNumber, 0)
      assert.equal(payload.maxProcessorNumber, 7)
      assert.equal(payload.profile, "ClosestStatic")
      return optimizedStatus
    },
    stateStore,
    logicalCpuCount: 16,
  })
  assert.deepEqual(actions, ["status", "apply"])
  assert.equal(result.optimized, true)
  assert.equal(result.restarted, true)
  assert.deepEqual(stateStore.read(), disabledStatus)
})

test("어댑터 재시작 직후 RSS 반영이 늦으면 상태를 다시 확인한다", async () => {
  const stateStore = createMemoryStateStore()
  const actions = []
  const result = await applyNicRssAffinity({
    applyChanges: true,
    runner: async action => {
      actions.push(action)
      if (actions.length === 1) return disabledStatus
      if (action === "apply") return disabledStatus
      return optimizedStatus
    },
    stateStore,
    logicalCpuCount: 16,
    wait: async () => {},
  })
  assert.deepEqual(actions, ["status", "apply", "status"])
  assert.equal(result.optimized, true)
})

test("복원 함수는 기존 RSS 꺼짐 상태를 복원하고 백업을 제거한다", async () => {
  const stateStore = createMemoryStateStore(disabledStatus)
  const actions = []
  const result = await restoreNicRssAffinity({
    applyChanges: true,
    runner: async (action, payload) => {
      actions.push(action)
      if (action === "status") return optimizedStatus
      assert.equal(action, "restore")
      assert.equal(payload.enabled, false)
      return disabledStatus
    },
    stateStore,
  })
  assert.equal(result.restored, true)
  assert.equal(result.restarted, true)
  assert.deepEqual(actions, ["status", "restore"])
  assert.equal(stateStore.read(), null)
})

test("NIC가 이미 원본 상태면 인터페이스를 재시작하지 않는다", async () => {
  const stateStore = createMemoryStateStore(disabledStatus)
  const actions = []
  const result = await restoreNicRssAffinity({
    applyChanges: true,
    runner: async action => {
      actions.push(action)
      assert.equal(action, "status")
      return disabledStatus
    },
    stateStore,
  })
  assert.equal(result.restored, true)
  assert.equal(result.restarted, false)
  assert.deepEqual(actions, ["status"])
  assert.equal(stateStore.read(), null)
})
