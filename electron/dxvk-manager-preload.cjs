const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("dxvkManager", {
  getStatus: () => ipcRenderer.invoke("dxvk:get-status"),
  checkUpdate: () => ipcRenderer.invoke("dxvk:check-update"),
  installUpdate: version => ipcRenderer.invoke("dxvk:install-update", version),
})
