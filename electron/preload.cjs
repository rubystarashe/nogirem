const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("nogirem", {
  getStatus: () => ipcRenderer.invoke("optimization:get-status"),
  refreshNvidia: () => ipcRenderer.invoke("optimization:refresh-nvidia"),
  refreshNetwork: () => ipcRenderer.invoke("optimization:refresh-network"),
  refreshAffinity: () => ipcRenderer.invoke("optimization:refresh-affinity"),
  getAffinityRuntime: () => ipcRenderer.invoke("optimization:get-affinity-runtime"),
  refreshMemory: () => ipcRenderer.invoke("optimization:refresh-memory"),
  getMemoryRuntime: () => ipcRenderer.invoke("optimization:get-memory-runtime"),
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
  resetFrameBoost: () => ipcRenderer.invoke("optimization:reset-frame-boost"),
  onCloseRequested: callback => {
    const listener = () => callback()
    ipcRenderer.on("application:close-requested", listener)
    return () => ipcRenderer.removeListener("application:close-requested", listener)
  },
  beginStartupReveal: () => ipcRenderer.invoke("application:begin-startup-reveal"),
  requestClose: () => ipcRenderer.invoke("application:request-close"),
  openCharacterGuide: () => ipcRenderer.invoke("application:open-character-guide"),
  openDxvkManager: () => ipcRenderer.invoke("application:open-dxvk-manager"),
  openDxvkGuide: () => ipcRenderer.invoke("application:open-dxvk-guide"),
  confirmClose: action => ipcRenderer.invoke("application:confirm-close", action),
})
