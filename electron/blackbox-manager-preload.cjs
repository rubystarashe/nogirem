const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("blackboxManager", {
  requestClose: () => ipcRenderer.invoke("blackbox-manager:request-close"),
  getStatus: () => ipcRenderer.invoke("blackbox-manager:get-status"),
  setSetting: setting => ipcRenderer.invoke("blackbox-manager:set-setting", setting),
  saveClip: () => ipcRenderer.invoke("blackbox-manager:save-clip"),
  clearRecording: () => ipcRenderer.invoke("blackbox-manager:clear-recording"),
  openEditor: () => ipcRenderer.invoke("blackbox-manager:open-editor"),
  openFolder: () => ipcRenderer.invoke("blackbox-manager:open-folder"),
})
