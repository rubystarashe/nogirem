import { EventEmitter } from 'node:events'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { readFileSync } from 'node:fs'
import { format } from 'node:util'
import { createTransport } from './transport.mjs'
import { runPowerShellScript } from '../src/powershell.mjs'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const packageInfo = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
// stdout is reserved exclusively for the host protocol.
for (const name of ['log', 'info', 'debug', 'warn', 'error']) {
  console[name] = (...args) => process.stderr.write(`${format(...args)}\n`)
}
export const transport = createTransport(process.stdin, process.stdout)
const windows = new Map()
const handlers = new Map()
const listeners = new Map()
let markReady
const ready = new Promise(resolve => { markReady = resolve })
let nextWindowId = 0
let tray
let quitting = false
export const native = (method, params) => transport.request(method, params)
const notify = (method, params) => transport.notify('native', { method, params })
const cancelable = () => ({ defaultPrevented: false, preventDefault() { this.defaultPrevented = true } })

export const app = Object.assign(new EventEmitter(), {
  isPackaged: process.env.NOGIREM_PACKAGED === '1',
  getAppPath: () => root,
  getVersion: () => packageInfo.version,
  getName: () => packageInfo.productName,
  getLocale: () => Intl.DateTimeFormat().resolvedOptions().locale,
  getPath(name) {
    const home = homedir()
    const paths = {
      appData: process.env.APPDATA ?? join(home, 'AppData', 'Roaming'),
      userData: process.env.NOGIREM_USER_DATA ?? join(process.env.APPDATA ?? join(home, 'AppData', 'Roaming'), packageInfo.productName),
      exe: process.env.NOGIREM_DESKTOP_EXE,
      documents: join(home, 'Documents'), downloads: join(home, 'Downloads'),
      videos: join(home, 'Videos'), temp: process.env.TEMP,
      ...JSON.parse(process.env.NOGIREM_KNOWN_FOLDERS ?? '{}'),
    }
    if (!paths[name]) throw new Error(`Unknown native path: ${name}`)
    return paths[name]
  },
  whenReady: async () => {},
  // The desktop process owns the single-instance mutex, including helper mode.
  requestSingleInstanceLock: () => true,
  async getGPUInfo() {
    const { stdout } = await runPowerShellScript(`
      $devices = @(Get-CimInstance Win32_VideoController | ForEach-Object {
        $vendor = 0
        if ($_.PNPDeviceID -match 'VEN_([0-9A-F]{4})') { $vendor = [Convert]::ToInt32($matches[1], 16) }
        [ordered]@{ vendorId = $vendor; deviceString = $_.Name; driverVersion = $_.DriverVersion }
      })
      @{ gpuDevice = $devices } | ConvertTo-Json -Depth 4 -Compress
    `, { timeout: 15000 })
    return JSON.parse(stdout.trim())
  },
  relaunch: options => native('application.relaunch', options),
  exit(code = 0) { notify('application.exit', { code }); process.exitCode = code; transport.close() },
  quit() {
    if (quitting) return
    const event = cancelable()
    app.emit('before-quit', event)
    if (event.defaultPrevented) return
    quitting = true
    app.emit('will-quit')
    notify('application.exit', { code: 0 })
  },
})

class WebContents extends EventEmitter {
  constructor(owner) { super(); this.owner = owner }
  isDestroyed() { return this.owner.isDestroyed() }
  send(channel, ...args) { notify('window.event', { windowId: this.owner.id, channel, args }) }
  isLoadingMainFrame() { return this.owner.loading }
  executeJavaScript(script) { return native('window.eval', { windowId: this.owner.id, script }) }
  reload() { this.owner.loadFile(this.owner.file) }
  focus() { this.owner.focus() }
  setBackgroundThrottling(value) { notify('window.background-throttling', { windowId: this.owner.id, value }) }
  setWindowOpenHandler(handler) { this.openHandler = handler }
}

export class NativeWindow extends EventEmitter {
  constructor(options = {}) {
    super()
    this.id = ++nextWindowId
    this.options = options
    this.state = { visible: options.show !== false, opacity: options.opacity ?? 1, focused: false,
      minimized: false, enabled: true, focusable: options.focusable !== false,
      alwaysOnTop: !!options.alwaysOnTop, x: options.x ?? 0, y: options.y ?? 0,
      width: options.width ?? 640, height: options.height ?? 290 }
    this.webContents = new WebContents(this)
    this.loaded = false
    this.commands = []
    windows.set(this.id, this)
  }
  static fromWebContents(contents) { return windows.get(contents?.owner?.id) }
  static getAllWindows() { return [...windows.values()] }
  static getFocusedWindow() { return [...windows.values()].find(w => w.state.focused) }
  async loadFile(file) {
    this.file = file
    this.loading = true
    const options = { ...this.options, parent: this.options.parent?.id }
    await native('window.open', { windowId: this.id, file, options })
    this.loaded = true
    for (const [action, value] of this.commands.splice(0)) this.command(action, value)
    this.loading = false
    this.webContents.emit('did-finish-load')
    this.emit('ready-to-show')
  }
  async loadURL(url) {
    if (!url.startsWith('data:text/html')) throw new Error('Remote native windows are prohibited')
    return this.loadFile(url)
  }
  isDestroyed() { return !windows.has(this.id) }
  isMinimized() { return this.state.minimized }
  isVisible() { return this.state.visible }
  isFocused() { return this.state.focused }
  isFocusable() { return this.state.focusable }
  isEnabled() { return this.state.enabled }
  isAlwaysOnTop() { return this.state.alwaysOnTop }
  getTitle() { return this.options.title ?? packageInfo.productName }
  getOpacity() { return this.state.opacity }
  getPosition() { return [this.state.x, this.state.y] }
  getBounds() { const { x, y, width, height } = this.state; return { x, y, width, height } }
  command(action, value) {
    if (!this.loaded) { this.commands.push([action, value]); return }
    notify('window.command', { windowId: this.id, action, value })
  }
  show() { this.state.visible = true; this.command('show'); this.emit('show') }
  showInactive() { this.state.visible = true; this.command('showInactive'); this.emit('show') }
  hide() { this.state.visible = false; this.command('hide'); this.emit('hide') }
  restore() { this.state.minimized = false; this.command('restore'); this.emit('restore') }
  focus() { this.command('focus') }
  moveTop() { this.command('moveTop') }
  center() { this.command('center') }
  setOpacity(value) { this.state.opacity = value; this.command('opacity', value) }
  setEnabled(value) { this.state.enabled = value; this.command('enabled', value) }
  setFocusable(value) { this.state.focusable = value; this.command('focusable', value) }
  setAlwaysOnTop(value) { this.state.alwaysOnTop = value; this.command('alwaysOnTop', value) }
  setSkipTaskbar(value) { this.command('skipTaskbar', value) }
  setIgnoreMouseEvents(value) { this.command('ignoreMouseEvents', value) }
  setVisibleOnAllWorkspaces(value) { this.command('allWorkspaces', value) }
  setMinimumSize(width, height) { this.command('minimumSize', { width, height }) }
  setPosition(x, y) { Object.assign(this.state, { x, y }); this.command('position', { x, y }) }
  setBounds(bounds) { Object.assign(this.state, bounds); this.command('bounds', bounds) }
  close() {
    const event = cancelable()
    this.emit('close', event)
    if (!event.defaultPrevented) this.destroy()
  }
  destroy() {
    if (this.isDestroyed()) return
    windows.delete(this.id)
    this.command('destroy')
    this.emit('closed')
    if (!windows.size) app.emit('window-all-closed')
  }
}

export const ipcMain = {
  ready: () => markReady(),
  handle(channel, handler) {
    if (handlers.has(channel)) throw new Error(`Duplicate IPC channel: ${channel}`)
    handlers.set(channel, handler)
  },
  on(channel, handler) { listeners.set(channel, handler) },
}
if (process.argv.includes('--contract-test')) {
  transport.handle('contract', async () => { await ready; return { channels: [...handlers.keys()], events: [...listeners.keys()] } })
}
transport.handle('invoke', async ({ windowId, channel, args = [] }) => {
  await ready
  const window = windows.get(windowId)
  if (!window) throw new Error('Unknown native window')
  const handler = handlers.get(channel)
  if (!handler) throw new Error(`Unknown IPC channel: ${channel}`)
  return handler({ sender: window.webContents }, ...args)
})
transport.on('send', ({ windowId, channel, args = [] }) => {
  const window = windows.get(windowId)
  if (window) listeners.get(channel)?.({ sender: window.webContents }, ...args)
})
transport.on('window', ({ windowId, event, state }) => {
  const window = windows.get(windowId)
  if (!window) return
  if (state) Object.assign(window.state, state)
  if (event === 'close') window.close()
  else if (event === 'key') window.webContents.emit('before-input-event', cancelable(), state)
  else window.emit(event)
})
transport.on('tray', ({ event, index }) => {
  if (event === 'menu') tray?.menu?.[index]?.click?.()
  else tray?.emit(event)
})
transport.on('disconnect', () => {
  // Host death must stop watchers just as closing the application does.
  if (!quitting) { app.emit('will-quit'); process.exitCode = 1 }
})

export const Menu = { buildFromTemplate: items => items }
export class Tray extends EventEmitter {
  constructor(icon) { super(); tray = this; notify('tray.create', { icon }) }
  setImage(icon) { notify('tray.icon', { icon }) }
  setToolTip(text) { notify('tray.tooltip', { text }) }
  setContextMenu(items) {
    this.menu = items
    notify('tray.menu', { items: items.map(({ label, enabled = true }, index) => ({ label, enabled, index })) })
  }
  isDestroyed() { return this.destroyed ?? false }
  destroy() { this.destroyed = true; notify('tray.destroy', {}) }
}
export const dialog = Object.fromEntries(['showMessageBox', 'showOpenDialog', 'showSaveDialog'].map(method =>
  [method, (...args) => native(`dialog.${method}`, { windowId: args.length > 1 ? args[0]?.id : null, options: args.at(-1) })]))
dialog.showErrorBox = (title, content) => notify('dialog.error', { title, content })
export const shell = Object.fromEntries(['openExternal', 'openPath', 'showItemInFolder'].map(method =>
  [method, path => native(`shell.${method}`, { path })]))
// Updated by the host before service startup and whenever monitors change.
let displays = JSON.parse(process.env.NOGIREM_DISPLAYS ?? '[]')
let cursor = { x: 0, y: 0 }
transport.on('displays', value => { displays = value.displays; cursor = value.cursor })
const primaryDisplay = () => {
  if (!displays.length) throw new Error('Native display information is unavailable')
  return displays.find(d => d.primary) ?? displays[0]
}
const nearest = point => displays.find(d => point.x >= d.bounds.x && point.y >= d.bounds.y
  && point.x < d.bounds.x + d.bounds.width && point.y < d.bounds.y + d.bounds.height) ?? primaryDisplay()
export const screen = { getPrimaryDisplay: primaryDisplay, getCursorScreenPoint: () => cursor,
  getDisplayNearestPoint: nearest, getDisplayMatching: bounds => nearest({ x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 }) }
const protocols = new Map()
export const protocol = { registerSchemesAsPrivileged() {}, handle: (scheme, handler) => protocols.set(scheme, handler) }
transport.handle('protocol', async ({ url, method, headers }) => {
  const scheme = new URL(url).protocol.slice(0, -1)
  const handler = protocols.get(scheme)
  if (!handler) throw new Error('Unknown media protocol')
  const response = await handler(new Request(url, { method, headers }))
  return { status: response.status, headers: Object.fromEntries(response.headers), body: Buffer.from(await response.arrayBuffer()).toString('base64') }
})
export const net = { fetch: (...args) => fetch(...args) }
