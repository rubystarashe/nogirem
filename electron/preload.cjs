const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("nogirem", {
  getStatus: () => ipcRenderer.invoke("optimization:get-status"),
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
  runCpuReorder: () => ipcRenderer.invoke("optimization:run-cpu-reorder"),
  resetFrameBoost: () => ipcRenderer.invoke("optimization:reset-frame-boost"),
  onCloseRequested: callback => {
    const listener = () => callback()
    ipcRenderer.on("application:close-requested", listener)
    return () => ipcRenderer.removeListener("application:close-requested", listener)
  },
  beginStartupReveal: () => ipcRenderer.invoke("application:begin-startup-reveal"),
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
  openDxvkGuide: () => ipcRenderer.invoke("application:open-dxvk-guide"),
  getCreatorChannel: () => ipcRenderer.invoke("application:get-creator-channel"),
  getCreatorPromptDismissed: () => ipcRenderer.invoke("application:get-creator-prompt-dismissed"),
  dismissCreatorPrompt: () => ipcRenderer.invoke("application:dismiss-creator-prompt"),
  openCreatorChannel: () => ipcRenderer.invoke("application:open-creator-channel"),
  openDirectDonation: () => ipcRenderer.invoke("application:open-direct-donation"),
  confirmClose: action => ipcRenderer.invoke("application:confirm-close", action),
})
