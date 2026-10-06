const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("debuffAlert", {
  getStatus: () => ipcRenderer.invoke("debuff-alert:get-status"),
  startDrag: () => ipcRenderer.send("debuff-alert:drag-start"),
})
