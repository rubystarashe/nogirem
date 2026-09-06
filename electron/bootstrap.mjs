import { appendFileSync, mkdirSync, statSync, truncateSync } from "node:fs"
import { inspect } from "node:util"
import { join } from "node:path"
import { app, dialog } from "electron"

const logDirectory = process.env.APPDATA
  ? join(process.env.APPDATA, "마비노기 렘 부스터", "logs")
  : join(app.getPath("userData"), "logs")
const logPath = join(logDirectory, "startup.log")
const originalConsoleError = console.error.bind(console)

function formatValue(value) {
  if (value instanceof Error) return value.stack ?? value.message
  return typeof value === "string" ? value : inspect(value, { depth: 5 })
}

function writeLog(level, values) {
  try {
    mkdirSync(logDirectory, { recursive: true })
    try {
      if (statSync(logPath).size > 1024 * 1024) truncateSync(logPath)
    } catch {
    }
    const message = values.map(formatValue).join(" ")
    appendFileSync(logPath, `${new Date().toISOString()} ${level} ${message}\n`, "utf8")
  } catch {
  }
}

console.error = (...values) => {
  writeLog("ERROR", values)
  originalConsoleError(...values)
}
globalThis.__nogiremWriteStartupLog = (level, ...values) => {
  writeLog(level, values)
}

process.on("uncaughtException", error => {
  writeLog("UNCAUGHT", [error])
  try {
    dialog.showErrorBox("마비노기 렘 부스터 오류", error?.message ?? String(error))
  } catch {
  }
  app.exit(1)
})

process.on("unhandledRejection", reason => {
  writeLog("REJECTION", [reason])
})

writeLog("START", [`version=${app.getVersion()}`, `args=${process.argv.join(" ")}`])

try {
  await import("./main.mjs")
} catch (error) {
  writeLog("BOOT_FAILURE", [error])
  try {
    dialog.showErrorBox("마비노기 렘 부스터 시작 실패", error?.message ?? String(error))
  } catch {
  }
  app.exit(1)
}
