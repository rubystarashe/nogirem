import { appendFileSync, mkdirSync, statSync, truncateSync } from 'node:fs'
import { inspect } from 'node:util'
import { join } from 'node:path'
import { app, dialog } from './native-host.mjs'

const logDirectory = join(app.getPath('userData'), 'logs')
const logPath = join(logDirectory, 'startup.log')
function writeLog(level, values) {
  try {
    mkdirSync(logDirectory, { recursive: true })
    try { if (statSync(logPath).size > 1024 * 1024) truncateSync(logPath) } catch {}
    const message = values.map(value => value instanceof Error ? value.stack : typeof value === 'string' ? value : inspect(value, { depth: 5 })).join(' ')
    appendFileSync(logPath, `${new Date().toISOString()} ${level} ${message}\n`, 'utf8')
  } catch { /* Logging must not prevent application startup. */ }
}
const originalError = console.error
console.error = (...values) => { writeLog('ERROR', values); originalError(...values) }
globalThis.__nogiremWriteStartupLog = (level, ...values) => writeLog(level, values)
process.on('uncaughtException', error => {
  writeLog('UNCAUGHT', [error])
  try { dialog.showErrorBox('마비노기 렘 부스터 오류', error.message) } finally { app.exit(1) }
})
process.on('unhandledRejection', error => writeLog('REJECTION', [error]))
writeLog('START', [`Dioxus Desktop ${app.getVersion()}`, process.argv.slice(2)])
try { await import('./main.mjs') }
catch (error) {
  writeLog('BOOT_FAILURE', [error])
  try { dialog.showErrorBox('마비노기 렘 부스터 시작 실패', error.message) } finally { app.exit(1) }
}
