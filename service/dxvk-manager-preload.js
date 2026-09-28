const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("dxvkManager", {
  contentReady: timing => ipcRenderer.send("dxvk:content-ready", timing),
  requestClose: () => ipcRenderer.invoke("dxvk:request-close"),
  getStatus: () => ipcRenderer.invoke("dxvk:get-status"),
  checkUpdate: () => ipcRenderer.invoke("dxvk:check-update"),
  installUpdate: version => ipcRenderer.invoke("dxvk:install-update", version),
})
