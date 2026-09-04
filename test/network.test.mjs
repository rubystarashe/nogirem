import assert from "node:assert/strict"
import test from "node:test"
import {
  ensureFastPingForPrimaryInterface,
  ensureTcpAutoTuningNormal,
  isFastPingConfigured,
  isTcpAutoTuningNormal,
  restartPrimaryNetworkInterface,
} from "../src/network.mjs"

const baseStatus = {
  interfaceAlias: "Ethernet",
  interfaceIndex: 7,
  interfaceGuid: "{00000000-0000-0000-0000-000000000000}",
  nextHop: "192.0.2.1",
  routeMetric: 0,
  interfaceMetric: 25,
}

const normalAutoTuning = {
  settingName: "Internet",
  local: "Normal",
  groupPolicy: "NotConfigured",
  effective: "Normal",
}

test("두 레지스트리 값이 1이면 패스트핑 적용 상태로 판정한다", () => {
  assert.equal(isFastPingConfigured({
    ...baseStatus,
    TcpAckFrequency: 1,
    TCPNoDelay: 1,
  }), true)
  assert.equal(isFastPingConfigured({
    ...baseStatus,
    TcpAckFrequency: 1,
    TCPNoDelay: null,
  }), false)
})

test("이미 적용된 경우 레지스트리를 다시 쓰지 않는다", async () => {
  const calls = []
  const runner = async applyChanges => {
    calls.push(applyChanges)
    return {
      ...baseStatus,
      TcpAckFrequency: 1,
      TCPNoDelay: 1,
    }
  }

  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    runner,
  })

  assert.equal(result.configured, true)
  assert.equal(result.applied, false)
  assert.deepEqual(calls, [false])
})

test("미적용 상태에서는 두 값을 설정하고 결과를 검증한다", async () => {
  const calls = []
  const runner = async applyChanges => {
    calls.push(applyChanges)
    return {
      ...baseStatus,
      TcpAckFrequency: applyChanges ? 1 : null,
      TCPNoDelay: applyChanges ? 1 : 0,
    }
  }

  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    runner,
  })

  assert.equal(result.configured, true)
  assert.equal(result.applied, true)
  assert.deepEqual(calls, [false, true])
})

test("새 값을 적용한 경우 선택한 인터페이스를 재시작한다", async () => {
  const restarted = []
  const runner = async applyChanges => ({
    ...baseStatus,
    TcpAckFrequency: applyChanges ? 1 : null,
    TCPNoDelay: applyChanges ? 1 : null,
  })
  const restarter = async status => {
    restarted.push(status.interfaceIndex)
    return {
      interfaceAlias: status.interfaceAlias,
      interfaceIndex: status.interfaceIndex,
      status: "Up",
    }
  }

  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    restartAfterApply: true,
    runner,
    restarter,
  })

  assert.equal(result.restarted, true)
  assert.equal(result.restart.status, "Up")
  assert.deepEqual(restarted, [baseStatus.interfaceIndex])
})

test("이미 적용된 값은 자동 인터페이스 재시작을 유발하지 않는다", async () => {
  let restartCount = 0
  const status = {
    ...baseStatus,
    TcpAckFrequency: 1,
    TCPNoDelay: 1,
  }

  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    restartAfterApply: true,
    runner: async () => status,
    restarter: async () => {
      restartCount += 1
    },
  })

  assert.equal(result.restarted, false)
  assert.equal(restartCount, 0)
})

test("강제 재시작 함수는 지정된 인터페이스를 사용한다", async () => {
  const restart = await restartPrimaryNetworkInterface(baseStatus, {
    restarter: async status => ({
      interfaceAlias: status.interfaceAlias,
      interfaceIndex: status.interfaceIndex,
      status: "Up",
    }),
  })

  assert.deepEqual(restart, {
    interfaceAlias: "Ethernet",
    interfaceIndex: 7,
    status: "Up",
  })
})

test("드라이런에서는 미적용 값을 변경하지 않는다", async () => {
  const calls = []
  const runner = async applyChanges => {
    calls.push(applyChanges)
    return {
      ...baseStatus,
      TcpAckFrequency: null,
      TCPNoDelay: null,
    }
  }

  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: false,
    runner,
  })

  assert.equal(result.configured, false)
  assert.equal(result.applied, false)
  assert.deepEqual(calls, [false])
})

test("적용 후 값이 다르면 검증 오류를 발생시킨다", async () => {
  const runner = async () => ({
    ...baseStatus,
    TcpAckFrequency: 1,
    TCPNoDelay: 0,
  })

  await assert.rejects(
    ensureFastPingForPrimaryInterface({ applyChanges: true, runner }),
    /적용한 뒤 검증에 실패/,
  )
})

test("TCP 자동 조정은 Normal만 최적화 상태로 판정한다", () => {
  assert.equal(isTcpAutoTuningNormal(normalAutoTuning), true)
  assert.equal(isTcpAutoTuningNormal({
    ...normalAutoTuning,
    local: "Experimental",
    effective: "Experimental",
  }), false)
})

test("TCP 자동 조정이 Normal이면 다시 설정하지 않는다", async () => {
  const calls = []
  const result = await ensureTcpAutoTuningNormal({
    applyChanges: true,
    runner: async applyChanges => {
      calls.push(applyChanges)
      return normalAutoTuning
    },
  })

  assert.equal(result.optimized, true)
  assert.equal(result.applied, false)
  assert.deepEqual(calls, [false])
})

test("비최적화된 TCP 자동 조정을 Normal로 복구한다", async () => {
  const calls = []
  const result = await ensureTcpAutoTuningNormal({
    applyChanges: true,
    runner: async applyChanges => {
      calls.push(applyChanges)
      return applyChanges
        ? normalAutoTuning
        : {
            ...normalAutoTuning,
            local: "Experimental",
            effective: "Experimental",
          }
    },
  })

  assert.equal(result.optimized, true)
  assert.equal(result.applied, true)
  assert.deepEqual(calls, [false, true])
})

test("그룹 정책이 Normal 이외의 값을 강제하면 복구 실패로 처리한다", async () => {
  const policyStatus = {
    settingName: "Internet",
    local: "Normal",
    groupPolicy: "Disabled",
    effective: "Disabled",
  }

  await assert.rejects(
    ensureTcpAutoTuningNormal({
      applyChanges: true,
      runner: async () => policyStatus,
    }),
    /그룹 정책=Disabled/,
  )
})
