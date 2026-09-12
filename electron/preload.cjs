const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("nogirem", {
  getStatus: () => ipcRenderer.invoke("optimization:get-status"),
  onGraphicsStatusChanged: callback => {
    const listener = (_event, status) => callback(status)
    ipcRenderer.on("optimization:graphics-status-changed", listener)
    return () => ipcRenderer.removeListener("optimization:graphics-status-changed", listener)
  },
  onDxvkStatusChanged: callback => {
    const listener = (_event, status) => callback(status)
    ipcRenderer.on("optimization:dxvk-status-changed", listener)
    return () => ipcRenderer.removeListener("optimization:dxvk-status-changed", listener)
  },
  onBlackboxStatusChanged: callback => {
    const listener = (_event, status) => callback(status)
    ipcRenderer.on("application:blackbox-status-changed", listener)
    return () => ipcRenderer.removeListener("application:blackbox-status-changed", listener)
  },
  refreshGraphics: () => ipcRenderer.invoke("optimization:refresh-graphics"),
  refreshNvidia: () => ipcRenderer.invoke("optimization:refresh-nvidia"),
  refreshNetwork: () => ipcRenderer.invoke("optimization:refresh-network"),
  refreshAffinity: () => ipcRenderer.invoke("optimization:refresh-affinity"),
  getAffinityRuntime: () => ipcRenderer.invoke("optimization:get-affinity-runtime"),
  getDxvkRuntimeStatus: () => ipcRenderer.invoke("optimization:get-dxvk-runtime-status"),
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
  refreshGameCpuCoreSetting: () => {
    return ipcRenderer.invoke("optimization:refresh-game-cpu-core-setting")
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
  getInputGuardSetting: () => ipcRenderer.invoke("application:get-input-guard-setting"),
  setInputGuardSetting: enabled => {
    return ipcRenderer.invoke("application:set-input-guard-setting", enabled)
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
  getNotice: () => ipcRenderer.invoke("application:get-notice"),
  dismissNotice: id => ipcRenderer.invoke("application:dismiss-notice", id),
  getReportResponses: () => ipcRenderer.invoke("application:get-report-responses"),
  acknowledgeReportResponse: responseId => {
    return ipcRenderer.invoke("application:acknowledge-report-response", responseId)
  },
  onReportResponsesAvailable: callback => {
    const listener = (_event, responses) => callback(responses)
    ipcRenderer.on("application:report-responses-available", listener)
    return () => {
      ipcRenderer.removeListener("application:report-responses-available", listener)
    }
  },
  openNoticeLink: url => ipcRenderer.invoke("application:open-notice-link", url),
  onNoticeAvailable: callback => {
    const listener = (_event, notice) => callback(notice)
    ipcRenderer.on("application:notice-available", listener)
    return () => ipcRenderer.removeListener("application:notice-available", listener)
  },
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
