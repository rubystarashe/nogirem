import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import {
  queryMabinogiVerticalSync,
  setMabinogiVerticalSync,
} from "./game-graphics.mjs"

const execFileAsync = promisify(execFile)
const root = dirname(dirname(fileURLToPath(import.meta.url)))

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
    const [raw, gameVerticalSync] = await Promise.all([
      runHelper(false),
      queryGameVerticalSync(),
    ])
    return normalizeRadeonResult(raw, gameVerticalSync)
  }

  async function apply() {
    const raw = await runHelper(true)
    if (!raw.detected) {
      throw new Error(raw.reason ?? "AMD Radeon GPU 또는 드라이버를 찾지 못했습니다")
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

async function runNativeHelper(apply) {
  try {
    const { stdout } = await execFileAsync(
      resolveHelperPath(),
      apply ? ["--apply"] : [],
      {
        windowsHide: true,
        timeout: 30000,
        encoding: "utf8",
        maxBuffer: 1024 * 1024,
      },
    )
    return JSON.parse(stdout.trim())
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

const defaultManager = createRadeonManager({ runHelper: runNativeHelper })

export function checkRadeonSettings() {
  return defaultManager.check()
}

export function applyRadeonSettings() {
  return defaultManager.apply()
}
