import assert from "node:assert/strict"
import test from "node:test"
import { applyNvidiaSettingDefinitions } from "../src/nvidia.mjs"

function createApi() {
  let currentSettingId = null
  const calls = []
  return {
    calls,
    createSession(session) {
      session[0] = {}
      calls.push("create")
      return 0
    },
    destroySession() {
      calls.push("destroy")
      return 0
    },
    loadSettings() {
      calls.push("load")
      return 0
    },
    findApplication(_session, _candidate, profile) {
      profile[0] = {}
      return 0
    },
    setSetting(_session, _profile, setting) {
      currentSettingId = setting.readUInt32LE(4100)
      calls.push(`public:${currentSettingId}`)
      return 0
    },
    setPrivateSetting(_session, _profile, setting) {
      currentSettingId = setting.readUInt32LE(4100)
      calls.push(`private:${currentSettingId}`)
      return 0
    },
    saveSettings() {
      calls.push(`save:${currentSettingId}`)
      return currentSettingId === 2 ? -3 : 0
    },
  }
}

test("NVIDIA 설정을 개별 세션에 저장하고 지원하지 않는 항목만 건너뛴다", () => {
  const api = createApi()
  const outcome = applyNvidiaSettingDefinitions(api, ["Client.exe"], [
    { key: "publicOne", name: "공개 설정 1", id: 1, targetValue: 10 },
    { key: "privateOne", name: "비공개 설정", id: 2, targetValue: 20, privateSetting: true },
    { key: "publicTwo", name: "공개 설정 2", id: 3, targetValue: 30 },
  ])

  assert.deepEqual(outcome.unsupported.map(setting => setting.key), ["privateOne"])
  assert.deepEqual(outcome.failed, [])
  assert.deepEqual(api.calls, [
    "create",
    "load",
    "public:1",
    "save:1",
    "destroy",
    "create",
    "load",
    "private:2",
    "save:2",
    "destroy",
    "create",
    "load",
    "public:3",
    "save:3",
    "destroy",
  ])
})

test("비공개 setter가 없는 드라이버에서는 공개 설정을 계속 적용한다", () => {
  const api = createApi()
  api.setPrivateSetting = null
  const outcome = applyNvidiaSettingDefinitions(api, ["Client.exe"], [
    { key: "privateOne", name: "비공개 설정", id: 2, targetValue: 20, privateSetting: true },
    { key: "publicOne", name: "공개 설정", id: 1, targetValue: 10 },
  ])

  assert.deepEqual(outcome.unsupported.map(setting => setting.key), ["privateOne"])
  assert.equal(api.calls.includes("public:1"), true)
  assert.equal(api.calls.includes("save:1"), true)
})

test("NVAPI -1 저장 실패는 새 세션에서 두 번 재시도한다", () => {
  const api = createApi()
  let saveAttempts = 0
  api.saveSettings = () => {
    saveAttempts += 1
    api.calls.push(`save-attempt:${saveAttempts}`)
    return saveAttempts < 3 ? -1 : 0
  }

  const outcome = applyNvidiaSettingDefinitions(api, ["Client.exe"], [
    { key: "verticalSync", name: "수직 동기화", id: 1, targetValue: 10 },
  ])

  assert.equal(saveAttempts, 3)
  assert.deepEqual(outcome.failed, [])
  assert.equal(api.calls.filter(call => call === "create").length, 3)
  assert.equal(api.calls.filter(call => call === "destroy").length, 3)
})

test("NVAPI -1이 계속되면 해당 설정만 실패 처리하고 다음 설정을 적용한다", () => {
  const api = createApi()
  api.saveSettings = () => {
    api.calls.push("save-error")
    return -1
  }

  const outcome = applyNvidiaSettingDefinitions(api, ["Client.exe"], [
    { key: "verticalSync", name: "수직 동기화", id: 1, targetValue: 10 },
    { key: "maxFrameRate", name: "최대 프레임 속도", id: 3, targetValue: 400 },
  ])

  assert.deepEqual(
    outcome.failed.map(setting => setting.key),
    ["verticalSync", "maxFrameRate"],
  )
  assert.equal(api.calls.filter(call => call === "public:3").length, 3)
})
