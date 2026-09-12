import assert from "node:assert/strict"
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"
import AdmZip from "adm-zip"
import {
  collectDiagnosticEntries,
  createDiagnosticBundle,
  redactDiagnosticText,
} from "../src/diagnostic-bundle.mjs"

test("진단 문자열에서 개인정보와 인증값을 마스킹한다", () => {
  const source = [
    "C:\\Users\\Tester\\AppData\\Roaming\\nogirem\\startup.log",
    "address=192.168.0.15",
    "mail=tester@example.com",
    "\"accessToken\":\"secret-value\"",
    "Authorization: Bearer abc.def.ghi",
    "https://example.com/?token=private",
  ].join("\n")
  const redacted = redactDiagnosticText(source, {
    privatePaths: ["C:\\Users\\Tester\\AppData\\Roaming\\nogirem"],
  })
  assert.doesNotMatch(redacted, /Tester|192\.168\.0\.15|tester@example\.com|secret-value|abc\.def\.ghi|private/)
  assert.match(redacted, /%USER_DATA%/)
  assert.match(redacted, /%IP_ADDRESS%/)
  assert.match(redacted, /%EMAIL_ADDRESS%/)
  assert.match(redacted, /%REDACTED%/)
})

test("진단 수집은 로그와 상태만 포함하고 녹화 영상은 제외한다", async t => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-diagnostic-"))
  t.after(() => rm(directory, { recursive: true, force: true }))
  await mkdir(join(directory, "logs"), { recursive: true })
  await mkdir(join(directory, "blackbox", "Ring"), { recursive: true })
  await writeFile(join(directory, "logs", "startup.log"), "오류 10.0.0.3")
  await writeFile(join(directory, "blackbox", "blackbox-events.previous.log"), "이전 이벤트")
  await writeFile(join(directory, "blackbox", "recorder-metrics.log.previous"), "구버전 이전 지표")
  await writeFile(join(directory, "status.json.lock"), "{\"running\":true}")
  await writeFile(join(directory, "blackbox", "Ring", "chunk.mp4"), "video")
  await writeFile(join(directory, "helper.exe"), "binary")

  const entries = await collectDiagnosticEntries(directory)
  assert.deepEqual(entries.map(entry => entry.name).sort(), [
    "blackbox/blackbox-events.previous.log",
    "blackbox/recorder-metrics.log.previous",
    "logs/startup.log",
    "status.json.lock",
  ])
  assert.match(
    entries.find(entry => entry.name === "logs/startup.log").content,
    /%IP_ADDRESS%/,
  )
})

test("진단 ZIP에 시스템 요약과 마스킹된 로그 목록을 만든다", async t => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-diagnostic-"))
  t.after(() => rm(directory, { recursive: true, force: true }))
  await writeFile(join(directory, "status.json"), "{\"path\":\"C:\\\\Users\\\\Tester\\\\game\"}")
  const outputPath = join(directory, "diagnostics.zip")
  await createDiagnosticBundle({
    outputPath,
    userDataPath: directory,
    reportId: "ad2dd07f-23a9-40c2-8340-a45fd5c068fb",
    diagnostics: {
      reportId: "ad2dd07f-23a9-40c2-8340-a45fd5c068fb",
      generatedAt: "2026-09-12T13:27:00.000Z",
      version: "0.3.0",
      path: "C:\\Users\\Tester\\app.exe",
    },
    redactionOptions: {
      privatePaths: ["C:\\Users\\Tester"],
    },
  })

  const zip = new AdmZip(outputPath)
  const names = zip.getEntries().map(entry => entry.entryName)
  assert.ok(names.includes("diagnostics.json"))
  assert.ok(names.includes("report.json"))
  assert.ok(!names.includes("README.txt"))
  assert.ok(names.includes("included-files.json"))
  assert.ok(names.includes("files/status.json"))
  assert.match(zip.readAsText("diagnostics.json"), /"version": "0\.3\.0"/)
  assert.match(
    zip.readAsText("report.json"),
    /"reportId": "ad2dd07f-23a9-40c2-8340-a45fd5c068fb"/,
  )
  assert.doesNotMatch(zip.readAsText("files/status.json"), /Tester/)
})

test("고급 기능 UI와 제한된 preload IPC에 로그 추출을 연결한다", async () => {
  const [mainSource, preloadSource, appSource] = await Promise.all([
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
  ])
  assert.match(mainSource, /application:export-diagnostic-logs/)
  assert.match(mainSource, /BrowserWindow\.fromWebContents\(event\.sender\) !== primaryWindow/)
  assert.match(
    mainSource,
    /diagnosticResult\(\(\) => ensureFastPingForPrimaryInterface\(\)\)[\s\S]*network: \{[\s\S]*fastPing/,
  )
  assert.match(preloadSource, /exportDiagnosticLogs/)
  assert.match(appSource, /<h2>버그 리포트<\/h2>/)
  assert.match(appSource, /문제가 발생한 경우 로그 추출 파일을 전송해 주세요/)
  assert.ok(appSource.indexOf("<h2>버그 리포트</h2>") < appSource.indexOf("<h2>Windows 시작 시 트레이 실행</h2>"))
  assert.match(appSource, /onclick=\{exportDiagnosticLogs\}/)
})
