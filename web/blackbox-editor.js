const editor = document.querySelector(".editor")
const video = document.querySelector("video")
const loadingState = document.querySelector(".loading-state span")
const emptyState = document.querySelector(".empty-state")
const closeButton = document.querySelector(".window-close")
const anchorTime = document.querySelector(".anchor-time")
const addTimeButton = document.querySelector(".add-time")
const directTimeButton = document.querySelector(".direct-time")
const directLengthForm = document.querySelector(".direct-length")
const trackSecondsInput = document.querySelector("#track-seconds")
const extractSecondsInput = document.querySelector("#extract-seconds")
const timeline = document.querySelector(".timeline")
const guide = document.querySelector(".selection-guide")
const playhead = document.querySelector(".playhead")
const playToggle = document.querySelector(".play-toggle")
const currentTime = document.querySelector(".current-time")
const rangeTime = document.querySelector(".range-time")
const trackStart = document.querySelector(".track-start")
const previewRangeButton = document.querySelector(".preview-range")
const extractButton = document.querySelector(".extract")
const showOutputButton = document.querySelector(".show-output")
const notice = document.querySelector(".notice")
const embedded = new URLSearchParams(location.search).has("embedded")
const parentRequests = new Map()
let nextParentRequestId = 0

function requestParent(method, value) {
  const id = ++nextParentRequestId
  return new Promise((resolve, reject) => {
    parentRequests.set(id, { resolve, reject })
    window.parent.postMessage({
      source: "blackbox-editor",
      id,
      method,
      value,
    }, "*")
  })
}

window.addEventListener("message", event => {
  const data = event.data
  if (data?.source !== "blackbox-manager" || !parentRequests.has(data.id)) return
  const request = parentRequests.get(data.id)
  parentRequests.delete(data.id)
  if (data.error) request.reject(new Error(data.error))
  else request.resolve(data.result)
})

const editorBridge = embedded
  ? {
      getSession: () => requestParent("getSession"),
      setTrackSeconds: seconds => requestParent("setTrackSeconds", seconds),
      extract: range => requestParent("extract", range),
      showOutput: outputPath => requestParent("showOutput", outputPath),
    }
  : window.blackboxEditor

document.body.classList.toggle("embedded", embedded)

let requestedTrackSeconds = 60
let duration = 0
let selectionStart = 0
let selectionDuration = 30
let initialTrackLoaded = false
let busy = true
let timelineInteraction = null
let previewingSelection = false
let lastOutputPath = ""

function messageOf(error) {
  return error?.message?.replace(/^Error invoking remote method '[^']+': Error: /, "")
    ?? String(error)
}

function formatTime(seconds) {
  const safe = Math.max(0, Number(seconds) || 0)
  const hours = Math.floor(safe / 3600)
  const minutes = Math.floor((safe % 3600) / 60)
  const remainingSeconds = Math.floor(safe % 60)
  return hours > 0
    ? `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(remainingSeconds).padStart(2, "0")}`
    : `${String(minutes).padStart(2, "0")}:${String(remainingSeconds).padStart(2, "0")}`
}

function setNotice(text = "", error = false) {
  notice.textContent = text
  notice.classList.toggle("error", error)
}

function setBusy(value, text = "") {
  busy = value
  addTimeButton.disabled = value
  directTimeButton.disabled = value
  trackSecondsInput.disabled = value
  extractSecondsInput.disabled = value
  previewRangeButton.disabled = value || !duration
  extractButton.disabled = value || !duration
  if (text) {
    editor.className = "editor loading"
    loadingState.textContent = text
  }
}

function clampSelection() {
  selectionDuration = Math.max(1, Math.min(duration || 1, selectionDuration))
  selectionStart = Math.max(
    0,
    Math.min(Math.max(0, duration - selectionDuration), selectionStart),
  )
  extractSecondsInput.value = String(Math.round(selectionDuration * 10) / 10)
  extractSecondsInput.max = String(Math.max(1, Math.floor(duration)))
}

function renderTimeline() {
  const total = Math.max(duration, 0.001)
  clampSelection()
  guide.style.left = `${selectionStart / total * 100}%`
  guide.style.width = `${selectionDuration / total * 100}%`
  playhead.style.left = `${Math.min(total, video.currentTime || 0) / total * 100}%`
  currentTime.textContent = `${formatTime(video.currentTime)} / ${formatTime(duration)}`
  rangeTime.textContent = `${formatTime(selectionStart)} — ${formatTime(selectionStart + selectionDuration)}`
  trackStart.textContent = `-${formatTime(duration)}`
}

function loadVideo(url, preserveFromEnd = 0) {
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      video.removeEventListener("loadedmetadata", onLoaded)
      video.removeEventListener("error", onError)
    }
    const onLoaded = () => {
      cleanup()
      duration = Number.isFinite(video.duration) ? video.duration : 0
      if (!duration) {
        reject(new Error("재생 가능한 영상 길이를 확인하지 못했습니다"))
        return
      }
      if (!initialTrackLoaded) {
        selectionDuration = Math.min(30, duration)
        initialTrackLoaded = true
      }
      selectionStart = Math.max(0, duration - preserveFromEnd - selectionDuration)
      video.currentTime = selectionStart
      renderTimeline()
      resolve()
    }
    const onError = () => {
      cleanup()
      reject(new Error("편집 영상을 재생할 수 없습니다"))
    }
    video.addEventListener("loadedmetadata", onLoaded)
    video.addEventListener("error", onError)
    video.src = url
    video.load()
  })
}

async function applyTrack(result, preserveFromEnd = 0) {
  requestedTrackSeconds = result.requestedSeconds
  trackSecondsInput.value = String(requestedTrackSeconds)
  anchorTime.textContent = `${new Date(result.anchorAt).toLocaleTimeString("ko-KR")} 기준`
  await loadVideo(result.videoUrl, preserveFromEnd)
  editor.className = "editor ready"
  setBusy(false)
}

async function changeTrackSeconds(seconds) {
  const normalized = Math.max(30, Math.min(21600, Math.round(Number(seconds) || 60)))
  const preserveFromEnd = Math.max(0, duration - selectionStart - selectionDuration)
  setBusy(true, `같은 기준 시점에서 최근 ${formatTime(normalized)} 영상을 준비하고 있습니다`)
  setNotice("")
  video.pause()
  try {
    await applyTrack(
      await editorBridge.setTrackSeconds(normalized),
      preserveFromEnd,
    )
  } catch (error) {
    editor.className = duration ? "editor ready" : "editor failed"
    emptyState.textContent = messageOf(error)
    setBusy(false)
    setNotice(messageOf(error), true)
  }
}

function seekFromPointer(event) {
  if (!duration || busy) return
  const bounds = timeline.getBoundingClientRect()
  const ratio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width))
  const targetTime = ratio * duration
  video.currentTime = targetTime
  previewingSelection = (
    !video.paused
    && targetTime >= selectionStart
    && targetTime < selectionStart + selectionDuration
  )
  renderTimeline()
}

function beginTimelineInteraction(event) {
  if (!duration || busy || event.button !== 0) return
  event.preventDefault()
  const timelineBounds = timeline.getBoundingClientRect()
  const guideBounds = guide.getBoundingClientRect()
  const edgeSize = Math.min(10, Math.max(6, guideBounds.width / 4))
  const pointerSeconds = (event.clientX - timelineBounds.left)
    / timelineBounds.width
    * duration
  const insideGuide = event.clientX >= guideBounds.left
    && event.clientX <= guideBounds.right
  let mode = "seek"
  if (Math.abs(event.clientX - guideBounds.left) <= edgeSize) {
    mode = "resize-start"
  } else if (Math.abs(event.clientX - guideBounds.right) <= edgeSize) {
    mode = "resize-end"
  } else if (insideGuide && selectionDuration < duration - 0.01) {
    mode = "pending-move"
  }

  timelineInteraction = {
    mode,
    pointerId: event.pointerId,
    pointerStartX: event.clientX,
    pointerStartSeconds: pointerSeconds,
    originalStart: selectionStart,
    originalDuration: selectionDuration,
  }
  timeline.setPointerCapture?.(event.pointerId)
  if (mode === "seek" || mode === "pending-move") seekFromPointer(event)
  else guide.classList.add("dragging")
}

function updateTimelineInteraction(event) {
  const interaction = timelineInteraction
  if (!interaction || !duration || event.pointerId !== interaction.pointerId) return
  const bounds = timeline.getBoundingClientRect()
  const pointerSeconds = Math.max(
    0,
    Math.min(duration, (event.clientX - bounds.left) / bounds.width * duration),
  )
  let mode = interaction.mode
  if (mode === "seek") return
  if (mode === "pending-move") {
    if (Math.abs(event.clientX - interaction.pointerStartX) < 6) return
    mode = "move"
    interaction.mode = mode
    guide.classList.add("dragging")
  }

  if (mode === "resize-start") {
    const selectionEnd = interaction.originalStart + interaction.originalDuration
    selectionStart = Math.max(0, Math.min(selectionEnd - 1, pointerSeconds))
    selectionDuration = selectionEnd - selectionStart
  } else if (mode === "resize-end") {
    const selectionEnd = Math.max(
      interaction.originalStart + 1,
      Math.min(duration, pointerSeconds),
    )
    selectionStart = interaction.originalStart
    selectionDuration = selectionEnd - selectionStart
  } else if (mode === "move") {
    const delta = pointerSeconds - interaction.pointerStartSeconds
    selectionStart = Math.max(
      0,
      Math.min(
        duration - interaction.originalDuration,
        interaction.originalStart + delta,
      ),
    )
    selectionDuration = interaction.originalDuration
  }

  clampSelection()
  video.pause()
  video.currentTime = selectionStart
  previewingSelection = false
  renderTimeline()
}

function endTimelineInteraction(event) {
  if (
    !timelineInteraction
    || (event && event.pointerId !== timelineInteraction.pointerId)
  ) return
  if (event && timeline.hasPointerCapture?.(event.pointerId)) {
    timeline.releasePointerCapture(event.pointerId)
  }
  timelineInteraction = null
  guide.classList.remove("dragging")
}

async function previewSelection() {
  if (!duration || busy) return
  previewingSelection = true
  video.currentTime = selectionStart
  try {
    await video.play()
  } catch (error) {
    setNotice(`영상을 재생할 수 없습니다: ${messageOf(error)}`, true)
  }
}

async function extractSelection() {
  if (!duration || busy) return
  setBusy(true)
  setNotice("선택한 구간을 MP4로 추출하고 있습니다")
  lastOutputPath = ""
  showOutputButton.classList.remove("visible")
  try {
    const result = await editorBridge.extract({
      startSeconds: selectionStart,
      durationSeconds: selectionDuration,
    })
    lastOutputPath = result.outputPath
    showOutputButton.classList.add("visible")
    setNotice(`${result.fileName} 저장 완료`)
  } catch (error) {
    setNotice(messageOf(error), true)
  } finally {
    setBusy(false)
  }
}

video.addEventListener("timeupdate", () => {
  if (
    previewingSelection
    && video.currentTime >= selectionStart + selectionDuration
  ) {
    video.pause()
    video.currentTime = selectionStart + selectionDuration
    previewingSelection = false
  }
  renderTimeline()
})

video.addEventListener("play", () => {
  playToggle.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6.5 5h4v14h-4V5Zm7 0h4v14h-4V5Z"/></svg>'
  playToggle.setAttribute("aria-label", "일시정지")
})

video.addEventListener("pause", () => {
  playToggle.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4.5v15L19 12 7 4.5Z"/></svg>'
  playToggle.setAttribute("aria-label", "재생")
})

async function togglePlayback() {
  if (!duration || busy) return
  if (video.paused) {
    if (video.currentTime >= duration) video.currentTime = 0
    const selectionEnd = selectionStart + selectionDuration
    if (
      video.currentTime >= selectionEnd - 0.01
      && video.currentTime <= selectionEnd + 0.05
    ) {
      video.currentTime = selectionStart
    }
    previewingSelection = (
      video.currentTime >= selectionStart - 0.01
      && video.currentTime < selectionEnd - 0.01
    )
    try {
      await video.play()
      setNotice("")
    } catch (error) {
      setNotice(`영상을 재생할 수 없습니다: ${messageOf(error)}`, true)
    }
  } else {
    video.pause()
    previewingSelection = false
  }
}

playToggle.addEventListener("click", togglePlayback)
video.addEventListener("click", togglePlayback)

timeline.addEventListener("pointerdown", beginTimelineInteraction)
window.addEventListener("pointermove", updateTimelineInteraction)
window.addEventListener("pointerup", endTimelineInteraction)
window.addEventListener("pointercancel", () => endTimelineInteraction())

window.addEventListener("keydown", event => {
  if (
    event.code !== "Space"
    || ["INPUT", "BUTTON", "SELECT", "TEXTAREA"].includes(event.target?.tagName)
  ) return
  event.preventDefault()
  void togglePlayback()
})

window.addEventListener("message", event => {
  if (
    event.data?.source === "blackbox-manager-command"
    && event.data.command === "toggle-playback"
  ) {
    void togglePlayback()
  }
})

extractSecondsInput.addEventListener("change", () => {
  selectionDuration = Number(extractSecondsInput.value) || 1
  clampSelection()
  video.currentTime = selectionStart
  renderTimeline()
})

addTimeButton.addEventListener("click", () => {
  void changeTrackSeconds(requestedTrackSeconds + 30)
})

directTimeButton.addEventListener("click", () => {
  directLengthForm.classList.toggle("visible")
  if (directLengthForm.classList.contains("visible")) {
    trackSecondsInput.focus()
    trackSecondsInput.select()
  }
})

directLengthForm.addEventListener("submit", event => {
  event.preventDefault()
  directLengthForm.classList.remove("visible")
  void changeTrackSeconds(trackSecondsInput.value)
})

previewRangeButton.addEventListener("click", previewSelection)
extractButton.addEventListener("click", extractSelection)
showOutputButton.addEventListener("click", () => {
  if (lastOutputPath) void editorBridge.showOutput(lastOutputPath)
})
closeButton.addEventListener("click", () => {
  if (embedded) return
  closeButton.disabled = true
  void editorBridge.requestClose()
})

async function initialize() {
  try {
    if (!editorBridge) throw new Error("블랙박스 편집 연결을 찾지 못했습니다")
    await applyTrack(await editorBridge.getSession())
  } catch (error) {
    editor.className = "editor failed"
    emptyState.textContent = messageOf(error)
    setBusy(false)
    setNotice(messageOf(error), true)
  }
}

void initialize()
