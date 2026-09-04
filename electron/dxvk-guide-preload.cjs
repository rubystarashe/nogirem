const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("dxvkGuide", {
  requestClose: () => ipcRenderer.invoke("dxvk-guide:request-close"),
})
