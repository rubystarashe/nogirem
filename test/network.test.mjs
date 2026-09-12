import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  checkNetworkConnectivity,
  ensureFastPingForPrimaryInterface,
  ensureTcpAutoTuningNormal,
  isFastPingConfigured,
  isTcpAutoTuningNormal,
  restoreFastPingForInterface,
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

const [networkSource, electronMain, electronPreload, applicationView, applicationStyles] = await Promise.all([
  readFile(new URL("../src/network.mjs", import.meta.url), "utf8"),
  readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
  readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
  readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
  readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
])

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

test("기본 경로에 대응하는 어댑터가 없으면 패스트핑을 안전하게 생략한다", async () => {
  const calls = []
  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    restartAfterApply: true,
    runner: async applyChanges => {
      calls.push(applyChanges)
      return {
        supported: false,
        specialNetwork: true,
        reason: "특수 네트워크 환경으로 패스트핑 적용 생략",
      }
    },
    restarter: async () => {
      throw new Error("특수 인터페이스를 재시작하면 안 됩니다")
    },
  })

  assert.equal(result.supported, false)
  assert.equal(result.configured, false)
  assert.equal(result.restarted, false)
  assert.equal(result.reason, "특수 네트워크 환경으로 패스트핑 적용 생략")
  assert.deepEqual(calls, [false])
})

test("가상 어댑터나 타사 필터 환경에서는 패스트핑 적용을 차단한다", async () => {
  const calls = []
  const result = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    runner: async applyChanges => {
      calls.push(applyChanges)
      return {
        ...baseStatus,
        compatible: false,
        compatibilityReason: "타사 네트워크 필터가 연결됨",
        TcpAckFrequency: null,
        TCPNoDelay: null,
      }
    },
  })

  assert.equal(result.compatibilityBlocked, true)
  assert.equal(result.reason, "타사 네트워크 필터가 연결됨")
  assert.deepEqual(calls, [false])
})

test("Npcap·SeLow와 nProtect 및 VMware 브리지 필터는 패스트핑 호환성 차단에서 제외한다", () => {
  assert.match(
    networkSource,
    /captureBindings[\s\S]*insecure_npcap\|npcap\(\?:_wifi\)\?\|selow[\s\S]*compatibleSecurityBindings[\s\S]*inca_tkfwfv[\s\S]*compatibleVirtualizationBindings[\s\S]*vmware_bridge[\s\S]*blockingThirdPartyBindings/,
  )
  assert.match(
    networkSource,
    /notmatch "\(\?i\)\^\(insecure_npcap\|npcap\(\?:_wifi\)\?\|selow\|inca_tkfwfv\|vmware_bridge\)\$"/,
  )
  assert.match(
    networkSource,
    /\$compatible = -not \$isVirtual -and \$blockingThirdPartyBindings\.Count -eq 0/,
  )
})

test("Realtek 네트워크 가속 필터는 차단하고 해제 방법을 안내한다", () => {
  assert.match(
    networkSource,
    /knownBlockingBindings[\s\S]*nt_rtf64\|nt_ndiswgc\|nt_ndextlag/,
  )
  assert.match(
    networkSource,
    /blockingThirdPartyBindings -contains "nt_rtf64"[\s\S]*Realtek LightWeight Filter \(NDIS6\.40\)를 해제/,
  )
  assert.match(
    networkSource,
    /interfaceDescription = \$adapter\.InterfaceDescription[\s\S]*knownBlockingBindings = \$knownBlockingBindings/,
  )
})

test("IP·기본 경로·게이트웨이·DNS·HTTPS가 모두 정상이면 연결 정상으로 판정한다", async () => {
  const result = await checkNetworkConnectivity({
    runner: async () => ({
      validIpv4: true,
      defaultRoute: true,
      gateway: true,
      dns: true,
      https: true,
    }),
  })

  assert.equal(result.healthy, true)
  assert.equal(result.attempts, 1)
})

test("연결 검사 실패는 지정 횟수만큼 재시도한다", async () => {
  let calls = 0
  const result = await checkNetworkConnectivity({
    attempts: 3,
    intervalMs: 0,
    runner: async () => {
      calls += 1
      return {
        validIpv4: true,
        defaultRoute: true,
        gateway: true,
        dns: calls >= 3,
        https: calls >= 3,
      }
    },
  })

  assert.equal(result.healthy, true)
  assert.equal(result.attempts, 3)
  assert.equal(calls, 3)
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

test("저장된 패스트핑 원래 값을 복원하고 인터페이스를 다시 시작한다", async () => {
  const target = {
    ...baseStatus,
    TcpAckFrequency: null,
    TCPNoDelay: 0,
  }
  const restarted = []
  const result = await restoreFastPingForInterface(target, {
    restartAfterRestore: true,
    runner: async received => ({
      ...received,
      supported: true,
    }),
    restarter: async status => {
      restarted.push(status.interfaceIndex)
      return { status: "Up" }
    },
  })

  assert.equal(result.restored, true)
  assert.equal(result.configured, false)
  assert.equal(result.restarted, true)
  assert.deepEqual(restarted, [baseStatus.interfaceIndex])
})

test("패스트핑 원래 DWORD 값은 PowerShell 명시식으로 복원한다", () => {
  assert.match(networkSource, /return `\(\[uint32\]\$\{number\}\)`/)
  assert.match(
    networkSource,
    /Restore-DwordValue "TcpAckFrequency" \$\{valueLiteral\(target\.TcpAckFrequency\)\}/,
  )
})

test("패스트핑 복원 결과가 원래 값과 다르면 실패한다", async () => {
  await assert.rejects(
    restoreFastPingForInterface({
      ...baseStatus,
      TcpAckFrequency: null,
      TCPNoDelay: null,
    }, {
      runner: async target => ({
        ...target,
        TcpAckFrequency: 1,
      }),
    }),
    /복원한 뒤 검증에 실패/,
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

test("패스트핑 원본 기록과 기록 없는 기본값 복원이 IPC와 UI에 연결된다", () => {
  assert.match(
    electronMain,
    /async function optimizeNetworkDirect\(\)[\s\S]*beforeFastPing[\s\S]*writeJsonAtomic\(statePath,[\s\S]*TcpAckFrequency: before\.TcpAckFrequency \?\? null/,
  )
  assert.match(
    electronMain,
    /const target = hasSavedTarget[\s\S]*TcpAckFrequency: null,[\s\S]*TCPNoDelay: null/,
  )
  assert.match(
    electronMain,
    /async function restoreNetworkDirect\(\)[\s\S]*ensureTcpAutoTuningNormal\(\)/,
  )
  assert.match(electronMain, /async function validateFastPingConnectivityAtStartup\(\)/)
  assert.match(electronMain, /attempts: 10,[\s\S]*intervalMs: 3000/)
  assert.match(electronMain, /if \(!connectivity\.healthy && fastPing\.configured\)/)
  assert.match(electronMain, /showNetworkRollbackDialog/)
  assert.match(electronMain, /createWindow\(\)[\s\S]*validateFastPingConnectivityAtStartup\(\)/)
  assert.match(electronMain, /optimization:restore-network/)
  assert.match(electronPreload, /restoreNetwork: \(\) => ipcRenderer\.invoke\("optimization:restore-network"\)/)
  assert.match(applicationView, /설정 되돌리기/)
  assert.match(applicationView, /TCP 자동 조정은 유지합니다/)
  assert.match(applicationView, /앱 시작 시 연결을 검사하며 이상이 반복되면 원래 설정으로 자동 복원/)
  assert.match(applicationStyles, /\.detail-restore/)
})
