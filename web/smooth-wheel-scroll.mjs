const standardFrameMs = 1000 / 60

function clamp(value, minimum, maximum) {
  return Math.max(minimum, Math.min(maximum, value))
}

export function createSmoothWheelScroller(
  getElement,
  {
    requestFrame = globalThis.requestAnimationFrame,
    cancelFrame = globalThis.cancelAnimationFrame,
    currentTime = () => globalThis.performance.now(),
  } = {},
) {
  let target = 0
  let frame = null
  let lastFrameAt = null

  function stop() {
    if (frame !== null) cancelFrame(frame)
    frame = null
    lastFrameAt = null
  }

  function reset(position = 0) {
    stop()
    target = position
  }

  function animate(frameTime) {
    const element = getElement()
    if (!element) {
      stop()
      return
    }
    const elapsed = Math.max(0, frameTime - (lastFrameAt ?? frameTime))
    lastFrameAt = frameTime
    const distance = target - element.scrollTop
    if (Math.abs(distance) < 0.5 || elapsed >= 100) {
      element.scrollTop = target
      frame = null
      lastFrameAt = null
      return
    }
    const progress = 1 - Math.pow(0.82, Math.max(1, elapsed / standardFrameMs))
    element.scrollTop += distance * progress
    frame = requestFrame(animate)
  }

  function handleWheel(event) {
    const element = getElement()
    if (event.ctrlKey || !element) return
    const maximum = element.scrollHeight - element.clientHeight
    if (maximum <= 0) return
    const unit = event.deltaMode === 1
      ? 16
      : event.deltaMode === 2
        ? element.clientHeight
        : 1
    const current = element.scrollTop
    const maximumPending = Math.max(96, element.clientHeight * 0.65)
    const pendingStart = frame === null
      ? current
      : clamp(target, current - maximumPending, current + maximumPending)
    const wheelDistance = clamp(
      event.deltaY * unit * 0.8,
      -maximumPending * 0.5,
      maximumPending * 0.5,
    )
    const next = clamp(
      clamp(pendingStart + wheelDistance, current - maximumPending, current + maximumPending),
      0,
      maximum,
    )
    if (next === pendingStart) return
    event.preventDefault()
    target = next
    if (frame === null) {
      lastFrameAt = currentTime()
      frame = requestFrame(animate)
    }
  }

  return { handleWheel, reset, stop }
}
