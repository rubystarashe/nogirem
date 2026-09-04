const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("dxvkGuide", {
  requestClose: () => ipcRenderer.invoke("dxvk-guide:request-close"),
  startDrag: (screenX, screenY) => {
    ipcRenderer.send("dxvk-guide:drag-start", { screenX, screenY })
  },
  moveDrag: (screenX, screenY) => {
    ipcRenderer.send("dxvk-guide:drag-move", { screenX, screenY })
  },
  endDrag: () => {
    ipcRenderer.send("dxvk-guide:drag-end")
  },
})
