import { closeWindowOnEscape } from "./window-escape.mjs"
// Explicit opt-in UI fixture. It never imports optimization code or writes user settings.
import { app, NativeWindow, ipcMain, Menu, Tray, transport } from './native-host.mjs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { defaultBlackboxSetting } from '../src/blackbox-settings.mjs'
const root = dirname(dirname(fileURLToPath(import.meta.url)))
const main = new NativeWindow({ width: 640, height: 290, frame: false, show: false, opacity: 0 })
const tray = new Tray(join(root, 'icon.ico'))
tray.setToolTip('마비노기 렘 부스터 — UI 검증 모드')
tray.setContextMenu(Menu.buildFromTemplate([
  { label: '열기', click: () => { main.show(); main.focus() } },
  { label: '종료', click: () => main.webContents.send('application:close-requested') },
]))
tray.on('click', () => main.show())
const status = { affinity: { ok: true, data: { running: true, gameActive: false } }, memory: { ok: true, data: { running: true } }, graphics: { ok: true, data: { detected: true, optimized: true } }, network: { ok: true, data: { fastPing: { optimized: true } } } }
const blackbox = { enabled: true, featureEnabled: true, durationSeconds: 125 }
status.graphics.data = {supported:true,detected:true,allMet:true,title:'NVIDIA 프로필',gpus:[{name:'NVIDIA GeForce RTX 4090'}],goalsList:['수직 동기화 끄기','최대 프레임 400 FPS','스레드 최적화','최고 성능 선호','저지연 모드 울트라'].map(label=>({label,met:true,supported:true}))}
status.network.data = {optimized:true,originalStateRecorded:true,fastPing:{supported:true,current:{interfaceAlias:'이더넷',TcpAckFrequency:1,TCPNoDelay:1}},tcpAutoTuning:{optimized:true}}
status.affinity.data.dxvk={state:'latest'}
status.affinity.data.characterSimplification={applied:true}
status.affinity.data.gameCoreSetting = { maxGameCoreCount: 4, gameCoreCount: 2, hybrid: true }
const turbo = { enabled: false, keys: [65], intervalMs: 10, installed: true, ignoreInitialDelay: false }
const input = { enabled: false, cursorScalePercent: 100, cursorWheelModifier: 'disabled' }
ipcMain.handle('application:get-launch-context', () => ({ optimizationStatus: status, startupMusicMuted: !process.argv.includes("--smoke-startup"), blackboxSetting: blackbox }))
ipcMain.handle('application:begin-startup-reveal', () => { main.setOpacity(1); main.show(); return true })
ipcMain.handle('application:complete-startup-animation', () => true)
ipcMain.handle('application:get-startup-tray-setting', () => ({ supported: false, enabled: false }))
ipcMain.handle('application:get-turbo-key-setting', () => turbo)
ipcMain.handle('application:get-input-guard-setting', () => input)
ipcMain.handle('application:set-turbo-key-setting', (_event, value) => { if (![1,3,5,10,20,30].includes(value.intervalMs) || typeof value.ignoreInitialDelay !== 'boolean') throw new Error('Invalid turbo setting'); return Object.assign(turbo, value) })
ipcMain.handle('application:set-input-guard-setting', (_event, value) => Object.assign(input, value))
ipcMain.handle('optimization:set-game-cpu-core-count', (_event, count) => { status.affinity.data.gameCoreSetting.gameCoreCount = count; return status.affinity.data })
ipcMain.handle('application:get-creator-channel', () => null)
ipcMain.handle('application:get-notice', () => null)
ipcMain.handle('application:get-report-responses', () => [])
ipcMain.handle('application:get-visual-activity', () => true)
ipcMain.handle('application:record-creator-prompt-display', () => false)
ipcMain.handle('optimization:get-affinity-runtime', () => status.affinity.data)
ipcMain.handle('optimization:get-memory-runtime', () => status.memory.data)
ipcMain.handle('optimization:get-status', () => status)
ipcMain.handle('optimization:set-frame-boost-enabled', (_event, { enabled }) => {
  status.affinity.data.running = enabled
  status.memory.data.running = enabled
  tray.setImage(join(root, enabled ? 'icon.ico' : 'icon-paused.png'))
  return { affinity: status.affinity.data, memory: status.memory.data }
})
ipcMain.handle('application:minimize-to-tray', () => { for (const window of NativeWindow.getAllWindows()) if (window !== main) window.close(); main.hide(); return true })
ipcMain.handle('application:request-close', () => { main.webContents.send('application:close-requested'); return true })
ipcMain.handle('application:confirm-close', (_event, action) => { if (action !== 'cancel') app.quit(); return { closing: action !== 'cancel' } })
for (const [channel, file] of [['character-guide', 'character-guide'], ['dxvk-guide', 'dxvk-guide'], ['dxvk-manager', 'dxvk-manager'], ['blackbox-manager', 'blackbox-manager'], ['blackbox-editor', 'blackbox-editor']]) {
  let window
  ipcMain.handle(`application:open-${channel}`, async () => {
    if (window && !window.isDestroyed()) { window.show(); return true }
    window = new NativeWindow({ width: 560, height: 430, frame: false, webPreferences: { preload: join(root, 'service', `${file}-preload.js`) } })
    closeWindowOnEscape(window, channel === "blackbox-manager" ? "blackbox-manager:escape-pressed" : "")
    await window.loadFile(join(root, `${file}.html`))
    return true
  })
  ipcMain.handle(`${channel === 'dxvk-manager' ? 'dxvk' : channel}:request-close`, () => { window?.close(); return true })
}
const dxvk = { installed: { installed: false, integrity: false }, deployment: { matchesCurrent: false }, releases: [], latest: null }
ipcMain.handle('dxvk:get-status', () => dxvk)
ipcMain.handle('dxvk:check-update', () => dxvk)
ipcMain.on('dxvk:content-ready', () => {})
ipcMain.handle('blackbox-manager:get-status', () => ({ ...defaultBlackboxSetting, storageDrives: [], resolvedQuality: '1080p', running: false, recording: false }))
ipcMain.handle('blackbox-manager:list-clips', () => [])
const track = { requestedSeconds: 900, anchorAt: Date.now(), segments: [], gaps: [], timelineDurationSeconds: 900 }
for (const prefix of ['blackbox-manager', 'blackbox-editor']) {
  ipcMain.handle(`${prefix}:${prefix === 'blackbox-manager' ? 'get-editor-session' : 'get-session'}`, () => track)
  ipcMain.handle(`${prefix}:report-playback`, () => true)
}
ipcMain.handle('blackbox-manager:set-page', () => true)
ipcMain.handle('blackbox-manager:fit-media', () => true)
transport.on('disconnect', () => process.exit(0))
ipcMain.ready()
await main.loadFile(join(root, 'index.html'))

ipcMain.handle('smoke:prewarm-hidden', async () => {
    const window = new NativeWindow({width:560,height:430,show:false,opacity:0,webPreferences:{preload:join(root,'service/dxvk-manager-preload.js')}})
    await window.loadFile(join(root,'dxvk-manager.html'))
    return true
})

ipcMain.handle("application:dismiss-notice", () => true)
ipcMain.handle("application:acknowledge-report-response", () => true)
