import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import {
  queryMabinogiVerticalSync,
  setMabinogiVerticalSync,
} from "./game-graphics.mjs"
import { runPowerShellScript } from "./powershell.mjs"

const execFileAsync = promisify(execFile)
const root = dirname(dirname(fileURLToPath(import.meta.url)))
const radeonGoals = [
  ["verticalSyncOff", "수직 동기화 항상 끄기"],
  ["enhancedSyncOff", "Enhanced Sync 끄기"],
  ["antiLagOn", "Radeon Anti-Lag 켜기"],
  ["chillOff", "Radeon Chill 끄기"],
]

function resolveHelperPath() {
  const executableRoot = root.includes("app.asar")
    ? root.replace("app.asar", "app.asar.unpacked")
    : root
  return join(
    executableRoot,
    "native",
    "radeon-helper",
    "bin",
    "radeon-helper.exe",
  )
}

export function normalizeRadeonResult(raw, gameVerticalSync = null) {
  if (!raw || typeof raw !== "object" || !Array.isArray(raw.gpus) || !Array.isArray(raw.goals)) {
    throw new Error("Radeon helper가 올바르지 않은 결과를 반환했습니다")
  }
  const goalsList = raw.goals.map(goal => ({
    key: String(goal.key ?? ""),
    label: String(goal.label ?? goal.key ?? ""),
    supported: goal.supported === true,
    met: goal.supported === true && goal.met === true,
    currentValue: goal.currentValue == null ? null : String(goal.currentValue),
  }))
  const verticalSync = goalsList.find(goal => goal.key === "verticalSyncOff")
  if (verticalSync?.supported && gameVerticalSync) {
    const gameSettingOff = gameVerticalSync.available && gameVerticalSync.enabled === false
    verticalSync.met = verticalSync.met && gameSettingOff
    if (!gameSettingOff) {
      verticalSync.currentValue = `${verticalSync.currentValue ?? "드라이버 확인"} · 게임 설정 켜기`
    }
  }
  const applicableGoals = goalsList.filter(goal => goal.supported)
  const detected = raw.detected === true && raw.gpus.length > 0
  return {
    supported: process.platform === "win32" && process.arch === "x64",
    detected,
    amd: detected,
    vendor: detected ? "amd" : null,
    vendorLabel: detected ? "AMD Radeon" : null,
    title: "AMD Radeon 전역 설정",
    scope: "global",
    scopeLabel: "Radeon GPU 전역 설정",
    persistentGlobal: detected,
    reason: raw.reason == null ? null : String(raw.reason),
    gpus: raw.gpus.map(gpu => ({ name: String(gpu.name ?? "AMD Radeon GPU") })),
    gameVerticalSync,
    goalsList,
    allMet: detected
      && raw.reason == null
      && raw.allMet === true
      && applicableGoals.length > 0
      && applicableGoals.every(goal => goal.met),
  }
}

export function createRadeonManager({
  runHelper,
  detectDevices = queryRadeonDevices,
  queryGameVerticalSync = queryMabinogiVerticalSync,
  setGameVerticalSync = setMabinogiVerticalSync,
}) {
  async function check() {
    if (process.platform !== "win32" || process.arch !== "x64") {
      return normalizeRadeonResult({
        detected: false,
        gpus: [],
        goals: [],
        reason: "Windows x64에서만 Radeon 설정을 조회할 수 있습니다",
      })
    }
    return normalizeRadeonDetection(await detectDevices())
  }

  async function apply() {
    const raw = await runHelper(true)
    if (!raw.detected) {
      throw new Error(raw.reason ?? "AMD Radeon GPU 또는 드라이버를 찾지 못했습니다")
    }
    if (raw.reason) {
      throw new Error(`${raw.reason} (최종 Radeon 3D 상태: ${JSON.stringify(raw.goals ?? [])})`)
    }
    if (raw.allMet !== true) {
      throw new Error(`Radeon 3D 목표 적용 검증 실패 (최종 Radeon 3D 상태: ${JSON.stringify(raw.goals ?? [])})`)
    }
    await setGameVerticalSync(false)
    const result = normalizeRadeonResult(raw, await queryGameVerticalSync())
    if (!result.allMet) {
      const unmet = result.goalsList
        .filter(goal => goal.supported && !goal.met)
        .map(goal => goal.label)
      throw new Error(`Radeon 최적화 적용 후 검증 실패: ${unmet.join(", ") || result.reason}`)
    }
    return result
  }

  return { check, apply }
}

export function normalizeRadeonDetection(raw) {
  const gpus = Array.isArray(raw?.gpus) ? raw.gpus : []
  const detected = gpus.length > 0
  return {
    supported: process.platform === "win32" && process.arch === "x64",
    detected,
    amd: detected,
    vendor: detected ? "amd" : null,
    vendorLabel: detected ? "AMD Radeon" : null,
    title: detected ? "AMD Radeon 전역 설정" : "그래픽 설정",
    scope: detected ? "global" : null,
    scopeLabel: detected ? "Radeon GPU 전역 설정" : null,
    persistentGlobal: detected,
    tuningProtected: detected,
    tuningProtectionNote: detected
      ? "시작·새로고침에서는 드라이버 API를 열지 않습니다. 적용할 때도 언더볼팅·오버클럭·전력 튜닝 API는 사용하지 않고 Radeon 3D 설정만 처리합니다."
      : null,
    reason: detected ? null : "AMD Radeon GPU를 찾지 못했습니다",
    gpus: gpus.map(gpu => ({
      name: String(gpu?.name || "AMD Radeon GPU"),
    })),
    gameVerticalSync: null,
    goalsList: detected
      ? radeonGoals.map(([key, label]) => ({
        key,
        label,
        supported: true,
        met: false,
        pending: true,
        currentValue: null,
      }))
      : [],
    allMet: false,
  }
}

async function queryRadeonDevices() {
  const { stdout } = await runPowerShellScript(`
$devices = @(Get-CimInstance Win32_VideoController | Where-Object { $_.PNPDeviceID -match 'VEN_1002' } | ForEach-Object { [ordered]@{ name = $_.Name } })
[ordered]@{ gpus = $devices } | ConvertTo-Json -Depth 4 -Compress
  `, { timeout: 15000 })
  return JSON.parse(stdout.trim())
}

async function runNativeHelper(apply) {
  const helperPath = resolveHelperPath()
  try {
    return await runRadeonHelper(
      args => execFileAsync(helperPath, args, {
        windowsHide: true,
        timeout: 30000,
        encoding: "utf8",
        maxBuffer: 1024 * 1024,
      }),
      apply,
    )
  } catch (error) {
    if (error?.code === "ENOENT") {
      return {
        detected: false,
        gpus: [],
        goals: [],
        reason: "Radeon 설정 helper를 찾지 못했습니다",
      }
    }
    if (error instanceof SyntaxError) {
      throw new Error("Radeon 설정 helper 응답을 해석할 수 없습니다")
    }
    throw error
  }
}

export async function runRadeonHelper(execute, apply) {
  const validate = result => {
    if (result?.tuningAccessed !== false) {
      throw new Error("Radeon helper가 언더볼팅 보호 계약을 확인하지 않았습니다")
    }
    return result
  }
  try {
    const { stdout } = await execute(apply ? ["--apply"] : [])
    return validate(JSON.parse(stdout.trim()))
  } catch (error) {
    if (error instanceof SyntaxError || error?.message?.includes("언더볼팅 보호 계약")) throw error
    const recoveredOutput = String(error?.stdout ?? "").trim()
    if (recoveredOutput) {
      try {
        return validate(JSON.parse(recoveredOutput))
      } catch (recoveredError) {
        if (recoveredError instanceof SyntaxError) throw recoveredError
        if (recoveredError?.message?.includes("언더볼팅 보호 계약")) throw recoveredError
      }
    }
    const numericCode = Number(error?.code)
    const exitCode = Number.isInteger(numericCode)
      ? `0x${(numericCode >>> 0).toString(16).padStart(8, "0").toUpperCase()}`
      : String(error?.code ?? error?.signal ?? "알 수 없음")
    throw new Error(`Radeon 설정 helper가 비정상 종료되었습니다 (종료 코드 ${exitCode})`)
  }
}

const defaultManager = createRadeonManager({ runHelper: runNativeHelper })

export function checkRadeonSettings() {
  return defaultManager.check()
}

export function applyRadeonSettings() {
  return defaultManager.apply()
}
