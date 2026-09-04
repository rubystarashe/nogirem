const { contextBridge, ipcRenderer } = require("electron")

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
