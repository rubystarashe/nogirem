const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("blackboxEditor", {
  requestClose: () => ipcRenderer.invoke("blackbox-editor:request-close"),
  getSession: () => ipcRenderer.invoke("blackbox-editor:get-session"),
  setEnabled: enabled => ipcRenderer.invoke("blackbox-editor:set-enabled", enabled),
  setTrackSeconds: seconds => {
    return ipcRenderer.invoke("blackbox-editor:set-track-seconds", seconds)
  },
  extract: range => ipcRenderer.invoke("blackbox-editor:extract", range),
  onExtractProgress: callback => {
    const listener = (_event, progress) => callback(Number(progress) || 0)
    ipcRenderer.on("blackbox-editor:extract-progress", listener)
    return () => ipcRenderer.removeListener("blackbox-editor:extract-progress", listener)
  },
  showOutput: outputPath => {
    return ipcRenderer.invoke("blackbox-editor:show-output", outputPath)
  },
})
