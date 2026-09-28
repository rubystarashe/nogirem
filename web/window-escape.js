// Let the focused renderer consume Escape before routing it to its native window.
// Keyboard events in an iframe do not bubble to the parent document.
export function attachWindowEscape(send, root = window) {
  const attached = new WeakSet()
  function attach(view) {
    if (!view || attached.has(view.document)) return
    const doc = view.document
    attached.add(doc)
    view.addEventListener('keydown', event => {
      if (event.key !== 'Escape' || event.repeat || doc.fullscreenElement) return
      // A timer runs after all listeners, including handlers registered after this one.
      setTimeout(() => {
        if (!event.defaultPrevented) send({ key: 'Escape' })
      }, 0)
    })
    function watchFrame(frame) {
      if (frame.dataset.nogiremEscapeAttached) return
      frame.dataset.nogiremEscapeAttached = 'true'
      const connect = () => {
        try { attach(frame.contentWindow) } catch { /* Cross-origin frames cannot be inspected. */ }
      }
      frame.addEventListener('load', connect)
      connect()
    }
    const scan = () => doc.querySelectorAll('iframe').forEach(watchFrame)
    new MutationObserver(scan).observe(doc.documentElement, { childList: true, subtree: true })
    scan()
  }
  attach(root)
}
