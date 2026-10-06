const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("overlayConfiguration", {
  startDrag: () => ipcRenderer.send("overlay-configuration:drag-start"),
  finish: () => ipcRenderer.invoke("overlay-configuration:finish"),
})
