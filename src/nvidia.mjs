import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { basename } from "node:path"
import koffi from "koffi"

const execFileAsync = promisify(execFile)

const NVAPI_OK = 0
const NVAPI_EXECUTABLE_NOT_FOUND = -166
const NVAPI_SETTING_NOT_FOUND = -160
const APPLICATION_SIZE = 16396
const SETTING_SIZE = 12320
const PROFILE_SIZE = 4116
const SETTING_ID_OFFSET = 4100
const SETTING_TYPE_OFFSET = 4104
const SETTING_LOCATION_OFFSET = 4108
const CURRENT_VALUE_OFFSET = 8220

const settingDefinitions = [
  {
    key: "verticalSync",
    name: "수직 동기화",
    id: 0x00A879CF,
    defaultValue: 0x60925292,
    targetValue: 0x08416747,
    values: new Map([
      [0x60925292, "3D 응용 프로그램 설정 사용"],
      [0x08416747, "끄기"],
      [0x47814940, "켜기"],
      [0x32610244, "1/2 새로 고침 빈도"],
      [0x71271021, "1/3 새로 고침 빈도"],
      [0x13245256, "1/4 새로 고침 빈도"],
      [0x18888888, "가상"],
    ]),
  },
  {
    key: "maxFrameRate",
    name: "최대 프레임 속도",
    id: 0x10835002,
    defaultValue: 0,
    targetValue: 400,
    format: value => value === 0 ? "끄기" : `${value} FPS`,
  },
  {
    key: "threadedOptimization",
    name: "스레드 최적화",
    id: 0x20C1221E,
    defaultValue: 0,
    targetValue: 1,
    values: new Map([
      [0, "자동"],
      [1, "켜기"],
      [2, "끄기"],
    ]),
  },
  {
    key: "powerManagement",
    name: "전원 관리 모드",
    id: 0x1057EB71,
    defaultValue: 5,
    targetValue: 1,
    values: new Map([
      [0, "적응형"],
      [1, "최고 성능 선호"],
      [2, "드라이버 제어"],
      [3, "일관된 성능 선호"],
      [4, "최소 전력 선호"],
      [5, "최적 전력"],
    ]),
  },
  {
    key: "lowLatencyCpl",
    name: "저지연 모드 제어판 상태",
    id: 0x0005F543,
    defaultValue: 0,
    targetValue: 2,
    privateSetting: true,
    values: new Map([
      [0, "끄기"],
      [1, "켜기"],
      [2, "울트라"],
    ]),
  },
  {
    key: "lowLatencyEnabled",
    name: "저지연 모드 드라이버 활성화",
    id: 0x10835000,
    defaultValue: 0,
    targetValue: 1,
    privateSetting: true,
    values: new Map([
      [0, "끄기"],
      [1, "켜기"],
    ]),
  },
  {
    key: "maxPreRenderedFrames",
    name: "최대 사전 렌더링 프레임",
    id: 0x007BA09E,
    defaultValue: 0,
    targetValue: 1,
    format: value => value === 0 ? "3D 응용 프로그램 설정 사용" : String(value),
  },
]

const settingLocations = [
  "프로그램 프로필",
  "전역 프로필",
  "기본 프로필",
  "드라이버 기본값",
]
const prototypeCache = new Map()

function makeVersion(size, version) {
  return size | (version << 16)
}

function decodeUtf16(buffer, start, length) {
  return buffer
    .subarray(start, start + length)
    .toString("utf16le")
    .split("\0", 1)[0]
}

function requireFunction(queryInterface, id, prototype, fallbackId) {
  let pointer = queryInterface(id)
  if (!pointer && fallbackId !== undefined) pointer = queryInterface(fallbackId)
  if (!pointer) throw new Error(`NVAPI 함수를 찾을 수 없습니다: 0x${id.toString(16)}`)
  return koffi.decode(pointer, prototype)
}

function getPrototype(signature) {
  if (!prototypeCache.has(signature)) {
    prototypeCache.set(signature, koffi.proto(signature))
  }
  return prototypeCache.get(signature)
}

async function queryNvidiaGpus() {
  try {
    const { stdout } = await execFileAsync(
      "nvidia-smi.exe",
      ["--query-gpu=name,driver_version", "--format=csv,noheader,nounits"],
      { windowsHide: true, timeout: 10000 },
    )
    return stdout
      .trim()
      .split(/\r?\n/)
      .filter(Boolean)
      .map(line => {
        const [name, driverVersion] = line.split(",").map(value => value.trim())
        return { name, driverVersion }
      })
  } catch {
    return []
  }
}

async function queryMabinogiVerticalSync() {
  try {
    const { stdout } = await execFileAsync(
      "reg.exe",
      [
        "query",
        "HKCU\\Software\\Nexon\\Mabinogi",
        "/v",
        "VerticalSync",
      ],
      { windowsHide: true, timeout: 10000 },
    )
    const match = stdout.match(/VerticalSync\s+(REG_DWORD|REG_SZ)\s+(0x[0-9a-f]+|\d+)/i)
    if (!match) {
      return {
        available: false,
        reason: "VerticalSync 레지스트리 값을 찾지 못했습니다",
      }
    }
    const rawValue = match[2]
    const value = Number.parseInt(rawValue, rawValue.toLowerCase().startsWith("0x") ? 16 : 10)
    return {
      available: true,
      value,
      enabled: value !== 0,
      registryType: match[1].toUpperCase(),
      location: "HKCU\\Software\\Nexon\\Mabinogi\\VerticalSync",
    }
  } catch (error) {
    return {
      available: false,
      reason: error.message,
    }
  }
}

async function setMabinogiVerticalSync(enabled) {
  await execFileAsync(
    "reg.exe",
    [
      "add",
      "HKCU\\Software\\Nexon\\Mabinogi",
      "/v",
      "VerticalSync",
      "/t",
      "REG_SZ",
      "/d",
      enabled ? "1" : "0",
      "/f",
    ],
    { windowsHide: true, timeout: 10000 },
  )
  const result = await queryMabinogiVerticalSync()
  if (!result.available || result.enabled !== enabled) {
    throw new Error("마비노기 수직 동기화 설정 검증에 실패했습니다")
  }
  return result
}

function openNvapi() {
  const library = koffi.load("nvapi64.dll")
  const queryInterface = library.func("void * __cdecl nvapi_QueryInterface(uint32_t id)")
  const initialize = requireFunction(
    queryInterface,
    0x0150E828,
    getPrototype("int32_t __cdecl NvAPI_Initialize()"),
  )
  const unload = requireFunction(
    queryInterface,
    0xD22BDD7E,
    getPrototype("int32_t __cdecl NvAPI_Unload()"),
  )
  const createSession = requireFunction(
    queryInterface,
    0x0694D52E,
    getPrototype("int32_t __cdecl NvAPI_DRS_CreateSession(_Out_ void **session)"),
  )
  const destroySession = requireFunction(
    queryInterface,
    0xDAD9CFF8,
    getPrototype("int32_t __cdecl NvAPI_DRS_DestroySession(void *session)"),
  )
  const loadSettings = requireFunction(
    queryInterface,
    0x375DBD6B,
    getPrototype("int32_t __cdecl NvAPI_DRS_LoadSettings(void *session)"),
  )
  const findApplication = requireFunction(
    queryInterface,
    0xEEE566B2,
    getPrototype(
      "int32_t __cdecl NvAPI_DRS_FindApplicationByName(void *session, str16 appName, _Out_ void **profile, _Inout_ void *application)",
    ),
  )
  const getBaseProfile = requireFunction(
    queryInterface,
    0xDA8466A0,
    getPrototype("int32_t __cdecl NvAPI_DRS_GetBaseProfile(void *session, _Out_ void **profile)"),
  )
  const getProfileInfo = requireFunction(
    queryInterface,
    0x61CD6FD6,
    getPrototype(
      "int32_t __cdecl NvAPI_DRS_GetProfileInfo(void *session, void *profile, _Inout_ void *profileInfo)",
    ),
  )
  const getSetting = requireFunction(
    queryInterface,
    0x73BF8338,
    getPrototype(
      "int32_t __cdecl NvAPI_DRS_GetSetting(void *session, void *profile, uint32_t settingId, _Inout_ void *setting)",
    ),
  )
  const getPrivateSetting = requireFunction(
    queryInterface,
    0xEA99498D,
    getPrototype(
      "int32_t __cdecl NvAPI_DRS_GetSettingInternal(void *session, void *profile, uint32_t settingId, _Inout_ void *setting, _Inout_ uint32_t *unknown)",
    ),
  )
  const setSetting = requireFunction(
    queryInterface,
    0x8A2CF5F5,
    getPrototype(
      "int32_t __cdecl NvAPI_DRS_SetSetting(void *session, void *profile, _In_ void *setting, uint32_t unknown1, uint32_t unknown2)",
    ),
    0x577DD202,
  )
  const saveSettings = requireFunction(
    queryInterface,
    0xFCBC7E14,
    getPrototype("int32_t __cdecl NvAPI_DRS_SaveSettings(void *session)"),
  )

  return {
    initialize,
    unload,
    createSession,
    destroySession,
    loadSettings,
    findApplication,
    getBaseProfile,
    getProfileInfo,
    getSetting,
    getPrivateSetting,
    setSetting,
    saveSettings,
  }
}

function checkStatus(operation, status) {
  if (status !== NVAPI_OK) throw new Error(`${operation} 실패: NVAPI ${status}`)
}

function findApplicationProfile(api, session, candidates) {
  for (const candidate of candidates) {
    const profile = [null]
    const application = Buffer.alloc(APPLICATION_SIZE)
    application.writeUInt32LE(makeVersion(APPLICATION_SIZE, 3), 0)
    const status = api.findApplication(session, candidate, profile, application)
    if (status === NVAPI_OK) {
      return {
        found: true,
        matchedApplication: candidate,
        profile: profile[0],
      }
    }
    if (status !== NVAPI_EXECUTABLE_NOT_FOUND) {
      throw new Error(`프로그램 프로필 조회 실패: NVAPI ${status}`)
    }
  }

  const profile = [null]
  checkStatus("기본 프로필 조회", api.getBaseProfile(session, profile))
  return {
    found: false,
    matchedApplication: null,
    profile: profile[0],
  }
}

function readProfileName(api, session, profile) {
  const profileInfo = Buffer.alloc(PROFILE_SIZE)
  profileInfo.writeUInt32LE(makeVersion(PROFILE_SIZE, 1), 0)
  const status = api.getProfileInfo(session, profile, profileInfo)
  if (status !== NVAPI_OK) return null
  return decodeUtf16(profileInfo, 4, 4096)
}

function readSetting(api, session, profile, definition) {
  const setting = Buffer.alloc(SETTING_SIZE)
  setting.writeUInt32LE(makeVersion(SETTING_SIZE, 1), 0)
  const unknown = [0]
  const status = definition.privateSetting
    ? api.getPrivateSetting(session, profile, definition.id, setting, unknown)
    : api.getSetting(session, profile, definition.id, setting)
  if (status === NVAPI_SETTING_NOT_FOUND) {
    const value = definition.defaultValue
    return {
      key: definition.key,
      name: definition.name,
      available: true,
      value,
      displayValue: definition.values?.get(value)
        ?? definition.format?.(value)
        ?? `0x${value.toString(16).padStart(8, "0")}`,
      location: "드라이버 기본값",
      explicit: false,
      status,
    }
  }
  checkStatus(`${definition.name} 조회`, status)

  const value = setting.readUInt32LE(CURRENT_VALUE_OFFSET)
  const locationIndex = setting.readUInt32LE(SETTING_LOCATION_OFFSET)
  return {
    key: definition.key,
    name: definition.name,
    available: true,
    value,
    displayValue: definition.values?.get(value)
      ?? definition.format?.(value)
      ?? `0x${value.toString(16).padStart(8, "0")}`,
    location: settingLocations[locationIndex] ?? `알 수 없음(${locationIndex})`,
    explicit: true,
  }
}

function writeSetting(api, session, profile, definition) {
  const setting = Buffer.alloc(SETTING_SIZE)
  setting.writeUInt32LE(makeVersion(SETTING_SIZE, 1), 0)
  setting.writeUInt32LE(definition.id, SETTING_ID_OFFSET)
  setting.writeUInt32LE(0, SETTING_TYPE_OFFSET)
  setting.writeUInt32LE(definition.targetValue, CURRENT_VALUE_OFFSET)
  checkStatus(
    `${definition.name} 적용`,
    api.setSetting(session, profile, setting, 0, 0),
  )
}

function resolveVerticalSync(verticalSync, gameVerticalSync) {
  if (!verticalSync?.available) {
    return {
      enabled: null,
      source: "판정 불가",
    }
  }

  if (verticalSync.value === 0x08416747) {
    return {
      enabled: false,
      source: "NVIDIA 프로그램 프로필 강제 끄기",
    }
  }
  if (
    verticalSync.value === 0x47814940
    || verticalSync.value === 0x32610244
    || verticalSync.value === 0x71271021
    || verticalSync.value === 0x13245256
  ) {
    return {
      enabled: true,
      source: "NVIDIA 프로그램 프로필",
    }
  }
  if (verticalSync.value === 0x60925292 && gameVerticalSync.available) {
    return {
      enabled: gameVerticalSync.enabled,
      source: "마비노기 내부 설정",
    }
  }
  return {
    enabled: null,
    source: "판정 불가",
  }
}

function buildChecks(settings, gameVerticalSync) {
  const values = Object.fromEntries(settings.map(setting => [setting.key, setting]))
  const verticalSync = resolveVerticalSync(values.verticalSync, gameVerticalSync)
  const maxFrameRate = values.maxFrameRate
  const threadedOptimization = values.threadedOptimization
  const powerManagement = values.powerManagement
  const lowLatencyCpl = values.lowLatencyCpl
  const lowLatencyEnabled = values.lowLatencyEnabled
  const maxPreRenderedFrames = values.maxPreRenderedFrames

  return {
    verticalSyncEnabled: verticalSync.enabled,
    verticalSyncSource: verticalSync.source,
    maxFrameRateEnabled: maxFrameRate?.available && maxFrameRate.value > 0,
    maxFrameRate: maxFrameRate?.available && maxFrameRate.value > 0
      ? maxFrameRate.value
      : null,
    threadedOptimizationEnabled: threadedOptimization?.available
      && threadedOptimization.value === 1,
    preferMaximumPerformance: powerManagement?.available
      && powerManagement.value === 1,
    ultraLowLatency: lowLatencyCpl?.available
      && lowLatencyCpl.value === 2
      && lowLatencyEnabled?.available
      && lowLatencyEnabled.value === 1
      && maxPreRenderedFrames?.available
      && maxPreRenderedFrames.value === 1,
    ultraLowLatencyCplState: lowLatencyCpl?.available && lowLatencyCpl.value === 2,
    ultraLowLatencyDriverEnabled: lowLatencyEnabled?.available
      && lowLatencyEnabled.value === 1,
    maxPreRenderedFramesOne: maxPreRenderedFrames?.available
      && maxPreRenderedFrames.value === 1,
  }
}

function buildGoalStatus(checks) {
  const goals = {
    verticalSyncOff: checks.verticalSyncEnabled === false,
    maxFrameRate400: checks.maxFrameRate === 400,
    threadedOptimizationOn: checks.threadedOptimizationEnabled === true,
    preferMaximumPerformance: checks.preferMaximumPerformance === true,
    ultraLowLatency: checks.ultraLowLatency === true,
  }
  return {
    ...goals,
    allMet: Object.values(goals).every(Boolean),
  }
}

export async function checkNvidiaProfile(gameExecutable) {
  if (process.platform !== "win32" || process.arch !== "x64") {
    return {
      supported: false,
      nvidia: false,
      reason: "Windows x64에서만 NVIDIA 프로필을 조회할 수 있습니다",
      gpus: [],
      settings: [],
      checks: {},
    }
  }

  const [gpus, gameVerticalSync] = await Promise.all([
    queryNvidiaGpus(),
    queryMabinogiVerticalSync(),
  ])
  if (!gpus.length) {
    return {
      supported: true,
      nvidia: false,
      reason: "NVIDIA GPU 또는 드라이버를 찾지 못했습니다",
      gpus,
      gameVerticalSync,
      settings: [],
      checks: {},
    }
  }

  let api
  try {
    api = openNvapi()
  } catch (error) {
    return {
      supported: true,
      nvidia: true,
      reason: `NVAPI 로드 실패: ${error.message}`,
      gpus,
      gameVerticalSync,
      settings: [],
      checks: {},
    }
  }

  const session = [null]
  checkStatus("NVAPI 초기화", api.initialize())
  checkStatus("DRS 세션 생성", api.createSession(session))

  try {
    checkStatus("DRS 설정 로드", api.loadSettings(session[0]))
    const candidates = [...new Set([
      gameExecutable,
      basename(gameExecutable ?? "Client.exe"),
    ].filter(Boolean))]
    const application = findApplicationProfile(api, session[0], candidates)
    const profileName = readProfileName(api, session[0], application.profile)
    const settings = settingDefinitions.map(definition =>
      readSetting(api, session[0], application.profile, definition))
    const checks = buildChecks(settings, gameVerticalSync)

    return {
      supported: true,
      nvidia: true,
      reason: null,
      gpus,
      profileFound: application.found,
      matchedApplication: application.matchedApplication,
      profileName,
      gameVerticalSync,
      settings,
      checks,
      goals: buildGoalStatus(checks),
      lowLatencyNote: "저지연 모드는 비공개 드라이버 상태값과 최대 사전 렌더링 프레임을 함께 조회합니다",
    }
  } finally {
    api.destroySession(session[0])
    api.unload()
  }
}

export async function applyNvidiaProfileGoals(gameExecutable) {
  if (process.platform !== "win32" || process.arch !== "x64") {
    throw new Error("Windows x64에서만 NVIDIA 프로필을 적용할 수 있습니다")
  }

  const gpus = await queryNvidiaGpus()
  if (!gpus.length) throw new Error("NVIDIA GPU 또는 드라이버를 찾지 못했습니다")

  const api = openNvapi()
  const session = [null]
  checkStatus("NVAPI 초기화", api.initialize())
  checkStatus("DRS 세션 생성", api.createSession(session))

  try {
    checkStatus("DRS 설정 로드", api.loadSettings(session[0]))
    const candidates = [...new Set([
      gameExecutable,
      basename(gameExecutable ?? "Client.exe"),
    ].filter(Boolean))]
    const application = findApplicationProfile(api, session[0], candidates)
    if (!application.found) {
      throw new Error("마비노기 NVIDIA 프로그램 프로필을 찾지 못했습니다")
    }
    for (const definition of settingDefinitions) {
      writeSetting(api, session[0], application.profile, definition)
    }
    checkStatus("DRS 설정 저장", api.saveSettings(session[0]))
  } finally {
    api.destroySession(session[0])
    api.unload()
  }

  await setMabinogiVerticalSync(false)
  const result = await checkNvidiaProfile(gameExecutable)
  if (!result.goals?.allMet) {
    const unmetGoals = Object.entries(result.goals ?? {})
      .filter(([key, met]) => key !== "allMet" && !met)
      .map(([key]) => key)
    const details = result.reason ?? (unmetGoals.join(", ") || "검증 결과 없음")
    throw new Error(`NVIDIA 최적화 적용 후 목표 설정 검증 실패: ${details}`)
  }
  return result
}

export function printNvidiaProfileStatus(result) {
  if (!result.nvidia) {
    console.log(`NVIDIA GPU: 감지되지 않음${result.reason ? ` (${result.reason})` : ""}`)
    return
  }

  console.log(`NVIDIA GPU: ${result.gpus.map(gpu => `${gpu.name} (${gpu.driverVersion})`).join(", ")}`)
  console.log(
    result.profileFound
      ? `마비노기 프로필: ${result.profileName ?? "이름 없음"} (${result.matchedApplication})`
      : `마비노기 프로필: 없음, ${result.profileName ?? "기본 프로필"} 설정 표시`,
  )
  if (result.reason) {
    console.log(`조회 오류: ${result.reason}`)
    return
  }

  for (const setting of result.settings) {
    console.log(
      setting.available
        ? `${setting.name}: ${setting.displayValue} [${setting.location}]`
        : `${setting.name}: 조회 불가`,
    )
  }

  const checks = result.checks
  const gameVerticalSync = result.gameVerticalSync
  console.log(
    gameVerticalSync.available
      ? `마비노기 내부 수직 동기화: ${gameVerticalSync.enabled ? "켜기" : "끄기"}`
      : `마비노기 내부 수직 동기화: 조회 불가`,
  )
  console.log(
    `수직 동기화 구성 판정: ${checks.verticalSyncEnabled === null ? "판정 불가" : (checks.verticalSyncEnabled ? "켜기" : "끄기")} [${checks.verticalSyncSource}]`,
  )
  console.log(`최대 프레임 속도 적용: ${checks.maxFrameRateEnabled ? `${checks.maxFrameRate} FPS` : "아니오"}`)
  console.log(`스레드 최적화 켜짐: ${checks.threadedOptimizationEnabled ? "예" : "아니오"}`)
  console.log(`최고 성능 선호: ${checks.preferMaximumPerformance ? "예" : "아니오"}`)
  console.log(`저지연 모드 울트라: ${checks.ultraLowLatency ? "예" : "아니오"}`)
}
