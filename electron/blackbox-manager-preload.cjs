const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("blackboxManager", {
  requestClose: () => ipcRenderer.invoke("blackbox-manager:request-close"),
  getStatus: () => ipcRenderer.invoke("blackbox-manager:get-status"),
  fitMedia: value => ipcRenderer.invoke("blackbox-manager:fit-media", value),
  setSetting: setting => ipcRenderer.invoke("blackbox-manager:set-setting", setting),
  saveClip: () => ipcRenderer.invoke("blackbox-manager:save-clip"),
  clearRecording: () => ipcRenderer.invoke("blackbox-manager:clear-recording"),
  openEditor: () => ipcRenderer.invoke("blackbox-manager:open-editor"),
  openFolder: () => ipcRenderer.invoke("blackbox-manager:open-folder"),
  listClips: () => ipcRenderer.invoke("blackbox-manager:list-clips"),
  openClip: fileName => ipcRenderer.invoke("blackbox-manager:open-clip", fileName),
  renameClip: (fileName, nextName) => {
    return ipcRenderer.invoke("blackbox-manager:rename-clip", fileName, nextName)
  },
  deleteClip: fileName => ipcRenderer.invoke("blackbox-manager:delete-clip", fileName),
  onEditorExtractProgress: callback => {
    const listener = (_event, progress) => callback(Number(progress) || 0)
    ipcRenderer.on("blackbox-editor:extract-progress", listener)
    return () => ipcRenderer.removeListener("blackbox-editor:extract-progress", listener)
  },
  editor: {
    getSession: () => ipcRenderer.invoke("blackbox-manager:get-editor-session"),
    setEnabled: enabled => ipcRenderer.invoke("blackbox-manager:set-enabled", enabled),
    fitMedia: value => ipcRenderer.invoke("blackbox-manager:fit-media", value),
    setTrackSeconds: seconds => {
      return ipcRenderer.invoke("blackbox-manager:set-track-seconds", seconds)
    },
    extract: range => ipcRenderer.invoke("blackbox-manager:extract", range),
    showOutput: outputPath => {
      return ipcRenderer.invoke("blackbox-manager:show-output", outputPath)
    },
  },
})
