const editor = document.querySelector(".editor")
const video = document.querySelector("video")
const preview = document.querySelector(".preview")
const gapPreview = document.querySelector(".gap-preview")
const loadingState = document.querySelector(".loading-state span")
const emptyState = document.querySelector(".empty-state")
const closeButton = document.querySelector(".window-close")
const anchorTime = document.querySelector(".anchor-time")
const addTimeButton = document.querySelector(".add-time")
const directTimeButton = document.querySelector(".direct-time")
const directLengthForm = document.querySelector(".direct-length")
const trackMinutesInput = document.querySelector("#track-minutes")
const trackSecondsInput = document.querySelector("#track-seconds")
const trackStatus = document.querySelector(".track-status")
const trackUsage = document.querySelector(".track-usage")
const trackDuration = document.querySelector(".track-duration")
const extractSecondsInput = document.querySelector("#extract-seconds")
const timeline = document.querySelector(".timeline")
const trackGapsLayer = document.querySelector(".track-gaps")
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
      fitMedia: value => requestParent("fitMedia", value),
      setTrackSeconds: seconds => requestParent("setTrackSeconds", seconds),
      extract: range => requestParent("extract", range),
      showOutput: outputPath => requestParent("showOutput", outputPath),
    }
  : window.blackboxEditor

document.body.classList.toggle("embedded", embedded)

let requestedTrackSeconds = 900
let duration = 0
let timelineDuration = 0
let trackSegments = []
let selectionStart = 0
let selectionDuration = 30
let timelineCursor = 0
let initialTrackLoaded = false
let busy = true
let timelineInteraction = null
let previewingSelection = false
let timelinePlaying = false
let playbackAnimationFrame = 0
let playbackStartedAt = 0
let playbackStartTimeline = 0
let trackGaps = []
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

function setTrackLengthInputs(totalSeconds) {
  const normalized = Math.max(30, Math.min(21600, Math.round(Number(totalSeconds) || 900)))
  trackMinutesInput.value = String(Math.floor(normalized / 60))
  trackSecondsInput.value = String(normalized % 60)
  return normalized
}

function normalizeTrackLengthInputs() {
  const minutes = Math.max(0, Math.round(Number(trackMinutesInput.value) || 0))
  const seconds = Math.max(0, Math.round(Number(trackSecondsInput.value) || 0))
  return setTrackLengthInputs(seconds >= 60 ? seconds : minutes * 60 + seconds)
}

function formatRecordedDuration(seconds) {
  const totalMinutes = Math.floor(Math.max(0, Number(seconds) || 0) / 60)
  if (totalMinutes < 1) return `${Math.floor(Math.max(0, Number(seconds) || 0))}초`
  if (totalMinutes < 60) return `${totalMinutes}분`
  return `${Math.floor(totalMinutes / 60)}시간 ${totalMinutes % 60}분`
}

function renderManagerStatus(value) {
  const bytesUsed = Math.max(0, Number(value?.bytesUsed) || 0)
  const capacityGb = Math.max(0, Number(value?.capacityGb) || 0)
  const durationSeconds = Math.max(0, Number(value?.durationSeconds) || 0)
  trackUsage.textContent = `현재 ${(bytesUsed / 1024 ** 3).toFixed(1)} / 최대 ${capacityGb}GB`
  trackDuration.textContent = durationSeconds > 0
    ? `${formatRecordedDuration(durationSeconds)} 녹화됨`
    : "녹화된 영상 없음"
  trackStatus.hidden = false
}

function setNotice(text = "", error = false) {
  notice.textContent = text
  notice.classList.toggle("error", error)
}

function setBusy(value, text = "") {
  busy = value
  addTimeButton.disabled = value
  directTimeButton.disabled = value
  trackMinutesInput.disabled = value
  trackSecondsInput.disabled = value
  extractSecondsInput.disabled = value
  previewRangeButton.disabled = value || !duration
  extractButton.disabled = value || !duration
  if (text) {
    editor.className = "editor loading"
    loadingState.textContent = text
  }
}

function segmentAtTimelineTime(time) {
  if (!trackSegments.length) return null
  return trackSegments.find(segment => (
    time >= segment.timelineStart
    && time <= segment.timelineStart + segment.duration
  )) ?? null
}

function clampSelection() {
  const total = timelineDuration || duration || 1
  selectionDuration = Math.max(1, Math.min(total, selectionDuration))
  selectionStart = Math.max(
    0,
    Math.min(total - selectionDuration, selectionStart),
  )
  extractSecondsInput.value = String(Math.round(selectionDuration * 10) / 10)
  extractSecondsInput.max = String(Math.max(1, Math.floor(total)))
}

function renderTimeline() {
  const total = Math.max(timelineDuration || duration, 0.001)
  clampSelection()
  guide.style.left = `${selectionStart / total * 100}%`
  guide.style.width = `${selectionDuration / total * 100}%`
  playhead.style.left = `${Math.min(total, timelineCursor) / total * 100}%`
  currentTime.textContent = `${formatTime(timelineCursor)} / ${formatTime(total)}`
  rangeTime.textContent = `${formatTime(selectionStart)} — ${formatTime(selectionStart + selectionDuration)}`
  trackStart.textContent = `-${formatTime(total)}`
}

function renderTrackGaps() {
  trackGapsLayer.replaceChildren()
  const total = timelineDuration || duration
  if (!total) return
  for (const gap of trackGaps) {
    const start = Math.max(0, Math.min(total, Number(gap?.startSeconds) || 0))
    const gapDuration = Math.max(0, Number(gap?.durationSeconds) || 0)
    if (!gapDuration || start >= total) continue
    const element = document.createElement("span")
    element.className = "track-gap"
    element.style.left = `${start / total * 100}%`
    element.style.width = `${Math.min(gapDuration, total - start) / total * 100}%`
    trackGapsLayer.append(element)
  }
}

function renderPlaybackButton() {
  playToggle.innerHTML = timelinePlaying
    ? '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6.5 5h4v14h-4V5Zm7 0h4v14h-4V5Z"/></svg>'
    : '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4.5v15L19 12 7 4.5Z"/></svg>'
  playToggle.setAttribute("aria-label", timelinePlaying ? "일시정지" : "재생")
}

function syncPreviewToTimeline() {
  const segment = segmentAtTimelineTime(timelineCursor)
  gapPreview.hidden = Boolean(segment)
  if (!segment) {
    if (!video.paused) video.pause()
    return
  }
  const targetMediaTime = segment.mediaStart
    + Math.max(0, Math.min(segment.duration, timelineCursor - segment.timelineStart))
  if (Math.abs(video.currentTime - targetMediaTime) > 0.12) {
    video.currentTime = targetMediaTime
  }
  if (timelinePlaying && video.paused) {
    void video.play().catch(error => {
      timelinePlaying = false
      renderPlaybackButton()
      setNotice(`영상을 재생할 수 없습니다: ${messageOf(error)}`, true)
    })
  }
}

function setTimelineCursor(time) {
  timelineCursor = Math.max(
    0,
    Math.min(timelineDuration || duration, Number(time) || 0),
  )
  syncPreviewToTimeline()
  renderTimeline()
}

function stopTimelinePlayback({ resetToSelection = false } = {}) {
  timelinePlaying = false
  cancelAnimationFrame(playbackAnimationFrame)
  playbackAnimationFrame = 0
  video.pause()
  if (resetToSelection) setTimelineCursor(selectionStart)
  renderPlaybackButton()
}

function updateTimelinePlayback(now) {
  if (!timelinePlaying) return
  timelineCursor = playbackStartTimeline + (now - playbackStartedAt) / 1000
  const playbackEnd = previewingSelection
    ? selectionStart + selectionDuration
    : timelineDuration
  if (timelineCursor >= playbackEnd) {
    timelineCursor = playbackEnd
    stopTimelinePlayback({ resetToSelection: previewingSelection })
    previewingSelection = false
    renderTimeline()
    return
  }
  syncPreviewToTimeline()
  renderTimeline()
  playbackAnimationFrame = requestAnimationFrame(updateTimelinePlayback)
}

function startTimelinePlayback() {
  if (timelineCursor >= timelineDuration) timelineCursor = 0
  timelinePlaying = true
  playbackStartTimeline = timelineCursor
  playbackStartedAt = performance.now()
  renderPlaybackButton()
  syncPreviewToTimeline()
  playbackAnimationFrame = requestAnimationFrame(updateTimelinePlayback)
}

function fitCurrentMedia() {
  if (video.videoWidth <= 0 || video.videoHeight <= 0) return
  const viewportWidth = preview.clientWidth
  const viewportHeight = preview.clientHeight
  preview.classList.add("media-ready")
  preview.style.setProperty("--media-aspect", `${video.videoWidth} / ${video.videoHeight}`)
  void editorBridge.fitMedia?.({
    page: "extract",
    mediaWidth: video.videoWidth,
    mediaHeight: video.videoHeight,
    viewportWidth,
    viewportHeight,
  })
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
      fitCurrentMedia()
      if (!trackSegments.length) {
        timelineDuration = duration
        trackSegments = [{
          timelineStart: 0,
          mediaStart: 0,
          duration,
        }]
      }
      if (!initialTrackLoaded) {
        const latestSegment = trackSegments.at(-1)
        selectionDuration = Math.min(30, latestSegment.duration)
        selectionStart = latestSegment.timelineStart
          + latestSegment.duration
          - selectionDuration
        initialTrackLoaded = true
      } else {
        selectionStart = Math.max(
          0,
          timelineDuration - preserveFromEnd - selectionDuration,
        )
        clampSelection()
      }
      setTimelineCursor(selectionStart)
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
  trackGaps = Array.isArray(result.gaps) ? result.gaps : []
  timelineDuration = Math.max(
    0,
    Number(result.timelineDurationSeconds) || requestedTrackSeconds,
  )
  trackSegments = Array.isArray(result.segments)
    ? result.segments
        .map(segment => ({
          timelineStart: Math.max(0, Number(segment?.timelineStartSeconds) || 0),
          mediaStart: Math.max(0, Number(segment?.mediaStartSeconds) || 0),
          duration: Math.max(0, Number(segment?.durationSeconds) || 0),
        }))
        .filter(segment => segment.duration >= 1)
    : []
  setTrackLengthInputs(requestedTrackSeconds)
  anchorTime.textContent = `${new Date(result.anchorAt).toLocaleTimeString("ko-KR")} 기준`
  await loadVideo(result.videoUrl, preserveFromEnd)
  renderTrackGaps()
  editor.className = "editor ready"
  setBusy(false)
}

async function changeTrackSeconds(seconds) {
  const normalized = Math.max(30, Math.min(21600, Math.round(Number(seconds) || 900)))
  const preserveFromEnd = Math.max(
    0,
    timelineDuration - selectionStart - selectionDuration,
  )
  setBusy(true, `같은 기준 시점에서 최근 ${formatTime(normalized)} 영상을 준비하고 있습니다`)
  setNotice("")
  stopTimelinePlayback()
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
  const targetTime = ratio * timelineDuration
  setTimelineCursor(targetTime)
  if (timelinePlaying) {
    playbackStartTimeline = timelineCursor
    playbackStartedAt = performance.now()
  }
  previewingSelection = (
    timelinePlaying
    && targetTime >= selectionStart
    && targetTime < selectionStart + selectionDuration
  )
}

function beginTimelineInteraction(event) {
  if (!duration || busy || event.button !== 0) return
  event.preventDefault()
  const timelineBounds = timeline.getBoundingClientRect()
  const guideBounds = guide.getBoundingClientRect()
  const edgeSize = Math.min(10, Math.max(6, guideBounds.width / 4))
  const pointerSeconds = (event.clientX - timelineBounds.left)
    / timelineBounds.width
    * timelineDuration
  const insideGuide = event.clientX >= guideBounds.left
    && event.clientX <= guideBounds.right
  let mode = "seek"
  if (Math.abs(event.clientX - guideBounds.left) <= edgeSize) {
    mode = "resize-start"
  } else if (Math.abs(event.clientX - guideBounds.right) <= edgeSize) {
    mode = "resize-end"
  } else if (insideGuide && selectionDuration < timelineDuration - 0.01) {
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
    Math.min(
      timelineDuration,
      (event.clientX - bounds.left) / bounds.width * timelineDuration,
    ),
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
      Math.min(timelineDuration, pointerSeconds),
    )
    selectionStart = interaction.originalStart
    selectionDuration = selectionEnd - selectionStart
  } else if (mode === "move") {
    const delta = pointerSeconds - interaction.pointerStartSeconds
    selectionDuration = interaction.originalDuration
    selectionStart = Math.max(
      0,
      Math.min(
        timelineDuration - selectionDuration,
        interaction.originalStart + delta,
      ),
    )
  }

  clampSelection()
  stopTimelinePlayback()
  setTimelineCursor(selectionStart)
  previewingSelection = false
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
  stopTimelinePlayback()
  setTimelineCursor(selectionStart)
  startTimelinePlayback()
}

async function extractSelection() {
  if (!duration || busy) return
  stopTimelinePlayback()
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

async function togglePlayback() {
  if (!duration || busy) return
  if (timelinePlaying) {
    stopTimelinePlayback()
    previewingSelection = false
    return
  }
  const selectionEnd = selectionStart + selectionDuration
  if (
    timelineCursor >= selectionEnd - 0.01
    && timelineCursor <= selectionEnd + 0.05
  ) {
    setTimelineCursor(selectionStart)
  }
  previewingSelection = (
    timelineCursor >= selectionStart - 0.01
    && timelineCursor < selectionEnd - 0.01
  )
  setNotice("")
  startTimelinePlayback()
}

playToggle.addEventListener("click", togglePlayback)
video.addEventListener("click", togglePlayback)
gapPreview.addEventListener("click", togglePlayback)

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
    embedded
    && event.source === window.parent
    && event.data?.source === "blackbox-manager-status"
  ) {
    renderManagerStatus(event.data.value)
  } else if (
    event.data?.source === "blackbox-manager-command"
    && event.data.command === "toggle-playback"
  ) {
    void togglePlayback()
  } else if (
    event.data?.source === "blackbox-manager-command"
    && event.data.command === "fit-media"
  ) {
    fitCurrentMedia()
  }
})

extractSecondsInput.addEventListener("change", () => {
  selectionDuration = Number(extractSecondsInput.value) || 1
  clampSelection()
  setTimelineCursor(selectionStart)
})

addTimeButton.addEventListener("click", () => {
  void changeTrackSeconds(requestedTrackSeconds + 30)
})

directTimeButton.addEventListener("click", () => {
  directLengthForm.classList.toggle("visible")
  if (directLengthForm.classList.contains("visible")) {
    trackMinutesInput.focus()
    trackMinutesInput.select()
  }
})

document.addEventListener("pointerdown", event => {
  if (
    directLengthForm.classList.contains("visible")
    && !directLengthForm.contains(event.target)
    && !directTimeButton.contains(event.target)
  ) {
    directLengthForm.classList.remove("visible")
  }
})

trackMinutesInput.addEventListener("change", normalizeTrackLengthInputs)
trackSecondsInput.addEventListener("change", normalizeTrackLengthInputs)

directLengthForm.addEventListener("submit", event => {
  event.preventDefault()
  directLengthForm.classList.remove("visible")
  void changeTrackSeconds(normalizeTrackLengthInputs())
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
