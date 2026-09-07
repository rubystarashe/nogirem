const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("nogirem", {
  getStatus: () => ipcRenderer.invoke("optimization:get-status"),
  onGraphicsStatusChanged: callback => {
    const listener = (_event, status) => callback(status)
    ipcRenderer.on("optimization:graphics-status-changed", listener)
    return () => ipcRenderer.removeListener("optimization:graphics-status-changed", listener)
  },
  refreshGraphics: () => ipcRenderer.invoke("optimization:refresh-graphics"),
  refreshNvidia: () => ipcRenderer.invoke("optimization:refresh-nvidia"),
  refreshNetwork: () => ipcRenderer.invoke("optimization:refresh-network"),
  refreshAffinity: () => ipcRenderer.invoke("optimization:refresh-affinity"),
  getAffinityRuntime: () => ipcRenderer.invoke("optimization:get-affinity-runtime"),
  refreshMemory: () => ipcRenderer.invoke("optimization:refresh-memory"),
  getMemoryRuntime: () => ipcRenderer.invoke("optimization:get-memory-runtime"),
  optimizeGraphics: () => ipcRenderer.invoke("optimization:optimize-graphics"),
  optimizeNvidia: () => ipcRenderer.invoke("optimization:optimize-nvidia"),
  optimizeNetwork: () => ipcRenderer.invoke("optimization:optimize-network"),
  restoreNetwork: () => ipcRenderer.invoke("optimization:restore-network"),
  setAffinityEnabled: options => {
    return ipcRenderer.invoke("optimization:set-affinity-enabled", options)
  },
  resetAffinity: () => ipcRenderer.invoke("optimization:reset-affinity"),
  setMemoryEnabled: enabled => {
    return ipcRenderer.invoke("optimization:set-memory-enabled", enabled)
  },
  setFrameBoostEnabled: options => {
    return ipcRenderer.invoke("optimization:set-frame-boost-enabled", options)
  },
  setGameCpuCoreCount: gameCoreCount => {
    return ipcRenderer.invoke("optimization:set-game-cpu-core-count", gameCoreCount)
  },
  runCpuReorder: () => ipcRenderer.invoke("optimization:run-cpu-reorder"),
  resetFrameBoost: () => ipcRenderer.invoke("optimization:reset-frame-boost"),
  onCloseRequested: callback => {
    const listener = () => callback()
    ipcRenderer.on("application:close-requested", listener)
    return () => ipcRenderer.removeListener("application:close-requested", listener)
  },
  beginStartupReveal: () => ipcRenderer.invoke("application:begin-startup-reveal"),
  getLaunchContext: () => ipcRenderer.invoke("application:get-launch-context"),
  exportDiagnosticLogs: () => ipcRenderer.invoke("application:export-diagnostic-logs"),
  openBugReportForm: () => ipcRenderer.invoke("application:open-bug-report-form"),
  getStartupTraySetting: () => ipcRenderer.invoke("application:get-startup-tray-setting"),
  setStartupTraySetting: enabled => {
    return ipcRenderer.invoke("application:set-startup-tray-setting", enabled)
  },
  getTurboKeySetting: () => ipcRenderer.invoke("application:get-turbo-key-setting"),
  downloadTurboKeyHelper: () => ipcRenderer.invoke("application:download-turbo-key-helper"),
  removeTurboKeyHelper: () => ipcRenderer.invoke("application:remove-turbo-key-helper"),
  setTurboKeySetting: setting => {
    return ipcRenderer.invoke("application:set-turbo-key-setting", setting)
  },
  getBlackboxSetting: () => ipcRenderer.invoke("application:get-blackbox-setting"),
  setBlackboxSetting: setting => {
    return ipcRenderer.invoke("application:set-blackbox-setting", setting)
  },
  setBlackboxEnabled: enabled => {
    return ipcRenderer.invoke("application:set-blackbox-enabled", enabled)
  },
  setBlackboxFeatureEnabled: enabled => {
    return ipcRenderer.invoke("application:set-blackbox-feature-enabled", enabled)
  },
  saveBlackboxClip: () => ipcRenderer.invoke("application:save-blackbox-clip"),
  clearBlackboxRecording: () => ipcRenderer.invoke("application:clear-blackbox-recording"),
  openBlackboxEditor: () => ipcRenderer.invoke("application:open-blackbox-editor"),
  openBlackboxFolder: () => ipcRenderer.invoke("application:open-blackbox-folder"),
  getVisualActivity: () => ipcRenderer.invoke("application:get-visual-activity"),
  onVisualActivityChanged: callback => {
    const listener = (_event, active) => callback(Boolean(active))
    ipcRenderer.on("application:visual-activity-changed", listener)
    return () => ipcRenderer.removeListener("application:visual-activity-changed", listener)
  },
  getUpdateState: () => ipcRenderer.invoke("application:get-update-state"),
  checkUpdate: () => ipcRenderer.invoke("application:check-update"),
  installUpdate: () => ipcRenderer.invoke("application:install-update"),
  onUpdateStateChanged: callback => {
    const listener = (_event, state) => callback(state)
    ipcRenderer.on("application:update-state-changed", listener)
    return () => ipcRenderer.removeListener("application:update-state-changed", listener)
  },
  requestClose: () => ipcRenderer.invoke("application:request-close"),
  minimizeToTray: () => ipcRenderer.invoke("application:minimize-to-tray"),
  openCharacterGuide: () => ipcRenderer.invoke("application:open-character-guide"),
  openDxvkManager: () => ipcRenderer.invoke("application:open-dxvk-manager"),
  openBlackboxManager: () => ipcRenderer.invoke("application:open-blackbox-manager"),
  openDxvkGuide: () => ipcRenderer.invoke("application:open-dxvk-guide"),
  getCreatorChannel: () => ipcRenderer.invoke("application:get-creator-channel"),
  getCreatorPromptDismissed: () => ipcRenderer.invoke("application:get-creator-prompt-dismissed"),
  recordCreatorPromptDisplay: () => ipcRenderer.invoke("application:record-creator-prompt-display"),
  dismissCreatorPrompt: () => ipcRenderer.invoke("application:dismiss-creator-prompt"),
  openCreatorChannel: () => ipcRenderer.invoke("application:open-creator-channel"),
  openDirectDonation: () => ipcRenderer.invoke("application:open-direct-donation"),
  openOperationPolicy: () => ipcRenderer.invoke("application:open-operation-policy"),
  confirmClose: action => ipcRenderer.invoke("application:confirm-close", action),
})
