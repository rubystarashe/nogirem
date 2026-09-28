export function closeWindowOnEscape(window, rendererEvent = "") {
  window.webContents.on("before-input-event", (event, input) => {
    if (input.type !== "keyDown" || input.key !== "Escape" || input.isAutoRepeat) return
    event.preventDefault()
    if (rendererEvent && !window.webContents.isDestroyed()) {
      window.webContents.send(rendererEvent)
    } else {
      window.close()
    }
  })
}

