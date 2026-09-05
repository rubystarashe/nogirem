const nvidiaGoalLabels = {
  verticalSyncOff: "수직 동기화 끄기",
  maxFrameRate400: "최대 프레임 400 FPS",
  threadedOptimizationOn: "스레드 최적화",
  preferMaximumPerformance: "최고 성능 선호",
  ultraLowLatency: "저지연 모드 울트라",
}

function normalizeNvidiaStatus(status) {
  const goals = Object.entries(nvidiaGoalLabels).map(([key, label]) => ({
    key,
    label,
    supported: true,
    met: status.goals?.[key] === true,
  }))
  return {
    ...status,
    detected: status.nvidia === true,
    vendor: status.nvidia ? "nvidia" : null,
    vendorLabel: status.nvidia ? "NVIDIA" : null,
    title: "NVIDIA 프로필",
    scope: "application",
    scopeLabel: "마비노기 프로그램 프로필",
    goalsList: goals,
    allMet: status.nvidia === true && goals.every(goal => goal.met),
  }
}

function unavailableStatus(nvidia, radeon) {
  const reasons = [nvidia?.reason, radeon?.reason].filter(Boolean)
  return {
    supported: process.platform === "win32" && process.arch === "x64",
    detected: false,
    vendor: null,
    vendorLabel: null,
    title: "그래픽 설정",
    scope: null,
    scopeLabel: null,
    gpus: [],
    goalsList: [],
    allMet: false,
    reason: reasons.join(" / ") || "지원되는 NVIDIA 또는 AMD Radeon GPU를 찾지 못했습니다",
  }
}

export function createGraphicsManager({
  checkNvidia,
  applyNvidia,
  checkRadeon,
  applyRadeon,
}) {
  let selectedVendor = null

  async function check(gameExecutable) {
    const nvidia = await checkNvidia(gameExecutable)
    if (nvidia.nvidia) {
      selectedVendor = "nvidia"
      return normalizeNvidiaStatus(nvidia)
    }

    const radeon = await checkRadeon()
    if (radeon.detected) {
      selectedVendor = "amd"
      return radeon
    }

    selectedVendor = null
    return unavailableStatus(nvidia, radeon)
  }

  async function apply(gameExecutable) {
    const current = await check(gameExecutable)
    if (selectedVendor === "nvidia") {
      return normalizeNvidiaStatus(await applyNvidia(gameExecutable))
    }
    if (selectedVendor === "amd") return applyRadeon()
    throw new Error(current.reason ?? "지원되는 그래픽 GPU를 찾지 못했습니다")
  }

  return { check, apply }
}

let defaultManagerPromise

async function getDefaultManager() {
  defaultManagerPromise ??= Promise.all([
    import("./nvidia.mjs"),
    import("./radeon.mjs"),
  ]).then(([nvidia, radeon]) => createGraphicsManager({
    checkNvidia: nvidia.checkNvidiaProfile,
    applyNvidia: nvidia.applyNvidiaProfileGoals,
    checkRadeon: radeon.checkRadeonSettings,
    applyRadeon: radeon.applyRadeonSettings,
  }))
  return defaultManagerPromise
}

export async function checkGraphics(gameExecutable) {
  return (await getDefaultManager()).check(gameExecutable)
}

export async function applyGraphicsGoals(gameExecutable) {
  return (await getDefaultManager()).apply(gameExecutable)
}

export function printGraphicsStatus(result) {
  if (!result.detected) {
    console.log(`그래픽 GPU: 감지되지 않음${result.reason ? ` (${result.reason})` : ""}`)
    return
  }
  console.log(`${result.vendorLabel} GPU: ${result.gpus.map(gpu => gpu.name).join(", ")}`)
  console.log(`적용 범위: ${result.scopeLabel}`)
  for (const goal of result.goalsList) {
    const value = goal.supported === false ? "지원 안 함" : (goal.met ? "완료" : "조정 필요")
    console.log(`${goal.label}: ${value}`)
  }
}
