const { contextBridge, ipcRenderer } = window.__nogiremBridge

contextBridge.exposeInMainWorld("characterGuide", {
  startDrag: (screenX, screenY) => {
    ipcRenderer.send("character-guide:drag-start", { screenX, screenY })
  },
  moveDrag: (screenX, screenY) => {
    ipcRenderer.send("character-guide:drag-move", { screenX, screenY })
  },
  endDrag: () => {
    ipcRenderer.send("character-guide:drag-end")
  },
})
