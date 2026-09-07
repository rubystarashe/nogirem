const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("blackboxEditor", {
  requestClose: () => ipcRenderer.invoke("blackbox-editor:request-close"),
  getSession: () => ipcRenderer.invoke("blackbox-editor:get-session"),
  setTrackSeconds: seconds => {
    return ipcRenderer.invoke("blackbox-editor:set-track-seconds", seconds)
  },
  extract: range => ipcRenderer.invoke("blackbox-editor:extract", range),
  showOutput: outputPath => {
    return ipcRenderer.invoke("blackbox-editor:show-output", outputPath)
  },
})
