const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("channelPing", {
  getStatus: () => ipcRenderer.invoke("channel-ping:get-status")
})
