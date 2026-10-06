const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("overlaySettings", {
  requestClose: () => ipcRenderer.invoke("overlay-settings:request-close"),
  getStatus: () => ipcRenderer.invoke("overlay-settings:get-status"),
  setSetting: setting => ipcRenderer.invoke("overlay-settings:set-setting", setting),
  detectDebuffRegion: () => ipcRenderer.invoke("overlay-settings:detect-debuff-region"),
  configureDebuffAlert: () => ipcRenderer.invoke("overlay-settings:configure-debuff-alert"),
})
