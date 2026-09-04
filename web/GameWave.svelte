<script>
  import { onMount } from "svelte"

  export let onstartupcomplete = () => {}
  export let onstartuphidden = () => {}
  export let onplaybackstart = () => {}

  const width = 640
  const height = 290
  const track = [1800, 2100, 2350, 2700, 2850, 3900, 4150, 4400, 4700, 4950]
  let canvas
  let context
  let image
  let imageCache
  let imageReady = false
  let startupAllowed = false
  let startupStopPending = false
  let audio
  let animationFrame
  let drawWakeTimer
  let hideTimer
  let audioStopTimer
  let ambientTimer
  let active = true
  let mode = "startup"
  let startedAt = 0
  let circles = []
  let pointerX = 0
  let pointerY = 0
  let smoothX = 0
  let smoothY = 0
  let playbackId = 0
  let darkBackground = false
  let backgroundTransition = null
  let ambientEnabled = true
  let ambientBlockedUntil = 0
  let pageVisible = true

  function createCircles() {
    return track.map((time, index) => index === track.length - 1
      ? {
          kind: "startup-finale",
          x: width / 2,
          y: 216.5,
          size: 600,
          time,
          stroke: 400,
          duration: 3500,
          innerDelay: 500,
          innerDuration: 3000,
        }
      : {
          x: Math.floor(Math.random() * 500) + 70,
          y: Math.floor(Math.random() * 150) + 70,
          size: Math.floor(Math.random() * 100) + 100,
          time,
        })
  }

  function currentTimelineElapsed() {
    return audio && !audio.paused
      ? Math.max(0, (audio.currentTime - 17.5) * 1000)
      : Date.now() - startedAt
  }

  function createImageCache() {
    const sourceImage = image
    const imageWidth = Math.ceil(width * 1.35)
    const imageHeight = Math.ceil(imageWidth * (sourceImage.height / sourceImage.width))
    imageCache = document.createElement("canvas")
    imageCache.width = imageWidth
    imageCache.height = imageHeight
    imageCache.getContext("2d").drawImage(sourceImage, 0, 0, imageWidth, imageHeight)
    image = null
  }

  function requestDraw() {
    window.clearTimeout(drawWakeTimer)
    drawWakeTimer = null
    if (animationFrame || !context || !imageCache) return
    animationFrame = requestAnimationFrame(draw)
  }

  function scheduleDraw(delayMs) {
    if (animationFrame || drawWakeTimer) return
    drawWakeTimer = window.setTimeout(() => {
      drawWakeTimer = null
      requestDraw()
    }, Math.max(0, delayMs))
  }

  function stopAmbientWaves() {
    window.clearTimeout(ambientTimer)
    ambientTimer = null
    circles = circles.filter(circle => circle.kind !== "ambient")
    if (pageVisible) requestDraw()
  }

  function scheduleAmbientWaves(delayMs = 1200) {
    stopAmbientWaves()
    if (!ambientEnabled || !pageVisible) return
    const boostTransitionPending = backgroundTransition?.targetDark === false
    if (mode !== "background" || (darkBackground && !boostTransitionPending)) return
    const startupRemaining = Math.max(0, ambientBlockedUntil - Date.now())
    const scheduledDelay = startupRemaining > 0
      ? startupRemaining + delayMs
      : delayMs
    ambientTimer = window.setTimeout(() => {
      ambientTimer = null
      const boostTransitionActive = backgroundTransition?.targetDark === false
      if (!ambientEnabled
        || !pageVisible
        || mode !== "background"
        || (darkBackground && !boostTransitionActive)) return
      const firstAnchor = {
        x: 80 + Math.random() * (width - 160),
        y: 55 + Math.random() * (height - 110),
      }
      let secondAnchor
      do {
        secondAnchor = {
          x: 80 + Math.random() * (width - 160),
          y: 55 + Math.random() * (height - 110),
        }
      } while (Math.hypot(
        secondAnchor.x - firstAnchor.x,
        secondAnchor.y - firstAnchor.y,
      ) < 160)
      const baseTime = currentTimelineElapsed()
      const firstTrackTime = track[0]
      const ambientCircles = track.map((time, index) => {
        const groupIndex = index % 5
        const anchor = index < 5 ? firstAnchor : secondAnchor
        const spread = 22 + groupIndex * 12
        const angle = Math.random() * Math.PI * 2
        const distance = Math.sqrt(Math.random()) * spread
        const size = 50 + Math.random() * 50
        return {
          kind: "ambient",
          x: anchor.x + Math.cos(angle) * distance,
          y: anchor.y + Math.sin(angle) * distance,
          size,
          time: baseTime + time - firstTrackTime,
          stroke: 3,
          duration: size * 12,
        }
      })
      circles = [
        ...circles.filter(circle => circle.kind !== "ambient"),
        ...ambientCircles,
      ]
      requestDraw()
      const rhythmDuration = track.at(-1) - firstTrackTime
      const nextWait = 1300 + Math.random() * 1700
      ambientTimer = window.setTimeout(() => scheduleAmbientWaves(0), rhythmDuration + nextWait)
    }, scheduledDelay)
  }

  function draw() {
    animationFrame = null
    if (!context || !imageCache) return
    if (mode === "background" && !pageVisible) return

    const now = Date.now()
    smoothX += (pointerX - smoothX) * 0.05
    smoothY += (pointerY - smoothY) * 0.05
    context.clearRect(0, 0, width, height)
    if (backgroundTransition) {
      const elapsed = now - backgroundTransition.startedAt
      if (elapsed >= backgroundTransition.duration) {
        darkBackground = backgroundTransition.targetDark
        backgroundTransition = null
      }
    }
    context.fillStyle = darkBackground ? "#000" : "#fff"
    context.fillRect(0, 0, width, height)
    if (backgroundTransition) {
      const progress = Math.min(
        1,
        Math.max(0, (now - backgroundTransition.startedAt) / backgroundTransition.duration),
      )
      context.save()
      context.beginPath()
      context.arc(
        backgroundTransition.x,
        backgroundTransition.y,
        backgroundTransition.radius * progress,
        0,
        Math.PI * 2,
      )
      context.clip()
      context.fillStyle = backgroundTransition.targetDark ? "#000" : "#fff"
      context.fillRect(0, 0, width, height)
      context.restore()
    }
    context.save()
    context.beginPath()
    const timelineElapsed = currentTimelineElapsed()
    const retainedCircles = []
    let activeWave = false
    let nextWaveDelay = Infinity

    for (const circle of circles) {
      const duration = circle.duration ?? circle.size * 8
      const elapsed = timelineElapsed - circle.time
      if (elapsed >= duration) continue
      retainedCircles.push(circle)
      if (elapsed <= 0) {
        nextWaveDelay = Math.min(nextWaveDelay, -elapsed)
        continue
      }
      activeWave = true

      const progress = elapsed / duration
      const radius = progress * circle.size
      const innerElapsed = elapsed - (circle.innerDelay ?? 0)
      const innerDuration = circle.innerDuration ?? duration
      const innerProgress = Math.max(0, Math.min(1, innerElapsed / innerDuration))
      const innerRadius = innerProgress * circle.size
      const stroke = (progress > 0.5 ? 2 - progress * 2 : progress * 2)
        * (circle.stroke ?? 6)
      context.moveTo(circle.x, circle.y)
      context.arc(circle.x, circle.y, radius + stroke, 0, Math.PI * 2)
      context.arc(circle.x, circle.y, innerRadius, 0, Math.PI * 2, true)
    }
    if (retainedCircles.length !== circles.length) circles = retainedCircles

    context.clip()
    const imageX = (width - imageCache.width) / 2 + smoothX / 20
    const imageY = -height * 0.05 + smoothY / 10
    context.drawImage(imageCache, imageX, imageY)
    context.restore()

    const audioPlaying = Boolean(audio && !audio.paused)
    if (audioPlaying) {
      const fadeOut = track.at(-1) - timelineElapsed + 1000
      audio.volume = Math.max(0, Math.min(1, timelineElapsed / 2000, fadeOut / 2000)) * 0.3
    }

    const pointerMoving = Math.abs(pointerX - smoothX) > 0.1
      || Math.abs(pointerY - smoothY) > 0.1
    if (pointerMoving || backgroundTransition || activeWave || audioPlaying) {
      requestDraw()
    } else if (Number.isFinite(nextWaveDelay)) {
      scheduleDraw(nextWaveDelay)
    }
  }

  function movePointer(event) {
    pointerX = event.clientX - width / 2
    pointerY = event.clientY - height / 2
    requestDraw()
  }

  function addClickWave(event) {
    if (
      event.target instanceof Element
      && event.target.closest(
        ".boost-text-area, .optimization-summary, .character-guide-link, .dxvk-update-link",
      )
    ) return
    const bounds = canvas?.getBoundingClientRect()
    if (!bounds) return
    circles = [
      ...circles,
      {
        x: ((event.clientX - bounds.left) / bounds.width) * width,
        y: ((event.clientY - bounds.top) / bounds.height) * height,
        size: Math.floor(Math.random() * 100) + 100,
        time: Date.now() - startedAt,
      },
    ]
    requestDraw()
  }

  export function makeActionWave(clientX, clientY, paused = false) {
    const bounds = canvas?.getBoundingClientRect()
    if (!bounds) return
    darkBackground = !paused
    const x = ((clientX - bounds.left) / bounds.width) * width
    const y = ((clientY - bounds.top) / bounds.height) * height
    const duration = paused ? 600 : 3000
    const size = paused ? 420 : 600
    const transitionDelay = paused ? 0 : 500
    const timelineElapsed = currentTimelineElapsed()
    circles = [
      ...circles.filter(circle => (
        circle.kind !== "action"
        || timelineElapsed - circle.time < circle.duration
      )),
      {
        kind: "action",
        x,
        y,
        size,
        time: Date.now() - startedAt,
        stroke: paused ? 32 : 400,
        duration: duration + transitionDelay,
        innerDelay: transitionDelay,
        innerDuration: duration,
      },
    ]
    backgroundTransition = {
      x,
      y,
      radius: size,
      targetDark: paused,
      startedAt: Date.now() + transitionDelay,
      duration,
    }
    if (paused) stopAmbientWaves()
    else if (ambientEnabled) scheduleAmbientWaves(1500)
    requestDraw()
    return { duration, radius: size, delay: transitionDelay }
  }

  export function setPaused(paused) {
    darkBackground = paused
    backgroundTransition = null
    if (paused) stopAmbientWaves()
    else if (ambientEnabled) scheduleAmbientWaves()
    requestDraw()
  }

  export function setAmbientEnabled(enabled) {
    if (ambientEnabled === enabled) {
      if (
        enabled
        && !ambientTimer
        && mode === "background"
        && (!darkBackground || backgroundTransition?.targetDark === false)
      ) {
        scheduleAmbientWaves()
      }
      return
    }
    ambientEnabled = enabled
    if (!enabled) stopAmbientWaves()
    else scheduleAmbientWaves()
  }

  export function setPageVisible(visible) {
    const nextVisible = Boolean(visible)
    if (pageVisible === nextVisible) return
    pageVisible = nextVisible
    if (!pageVisible) {
      window.clearTimeout(ambientTimer)
      ambientTimer = null
      circles = circles.filter(circle => circle.kind !== "ambient")
      if (mode === "background") {
        window.clearTimeout(drawWakeTimer)
        drawWakeTimer = null
        cancelAnimationFrame(animationFrame)
        animationFrame = null
      }
      return
    }
    requestDraw()
    if (
      ambientEnabled
      && mode === "background"
      && (!darkBackground || backgroundTransition?.targetDark === false)
    ) {
      scheduleAmbientWaves(300)
    }
  }

  function startPlayback(currentPlaybackId) {
    const nextAudio = audio
    nextAudio.volume = 0
    let animationStarted = false

    const startAnimation = () => {
      if (animationStarted || currentPlaybackId !== playbackId) return
      animationStarted = true
      onplaybackstart()
      circles = createCircles()
      startedAt = Date.now() - Math.max(0, (nextAudio.currentTime - 17.5) * 1000)
      const finalStartupWaveEnd = Math.max(
        6200,
        ...circles.map(circle => circle.time + (circle.duration ?? circle.size * 8)),
      )
      ambientBlockedUntil = startedAt + finalStartupWaveEnd
      cancelAnimationFrame(animationFrame)
      animationFrame = null
      requestDraw()
      hideTimer = window.setTimeout(() => {
        onstartupcomplete()
      }, 2500)
      audioStopTimer = window.setTimeout(() => {
        nextAudio.pause()
      }, 6200)
    }

    const playFromCue = () => {
      if (currentPlaybackId !== playbackId) return
      nextAudio.currentTime = 17.5
      nextAudio.addEventListener("playing", startAnimation, { once: true })
      void nextAudio.play().catch(error => {
        console.warn("게임 시작 음악을 재생하지 못했습니다", error)
        startAnimation()
      })
    }

    if (nextAudio.readyState >= HTMLMediaElement.HAVE_METADATA) playFromCue()
    else nextAudio.addEventListener("loadedmetadata", playFromCue, { once: true })
  }

  function playStartup() {
    playbackId += 1
    const currentPlaybackId = playbackId
    window.clearTimeout(hideTimer)
    window.clearTimeout(audioStopTimer)
    window.clearTimeout(drawWakeTimer)
    cancelAnimationFrame(animationFrame)
    animationFrame = null
    drawWakeTimer = null
    audio?.pause()
    circles = []
    ambientBlockedUntil = 0
    active = true
    mode = "startup"
    startPlayback(currentPlaybackId)
  }

  export function allowStartup() {
    if (startupAllowed) return
    startupAllowed = true
    if (imageReady) playStartup()
  }

  export function finishStartup() {
    if (mode !== "startup") return
    if (!imageReady) startupStopPending = true
    window.clearTimeout(hideTimer)
    hideTimer = window.setTimeout(() => {
      active = true
      mode = "background"
      onstartuphidden()
      requestDraw()
    }, 450)
  }

  onMount(() => {
    context = canvas.getContext("2d")
    audio = new Audio("./ost.mp3")
    audio.preload = "auto"
    audio.load()
    image = new Image()
    image.onload = () => {
      createImageCache()
      imageReady = true
      if (startupStopPending) finishStartup()
      else if (startupAllowed) playStartup()
    }
    image.onerror = error => {
      console.warn("시작 배경 이미지를 불러오지 못했습니다", error)
    }
    image.src = "./main3-optimized.jpg"
    return () => {
      playbackId += 1
      window.clearTimeout(hideTimer)
      window.clearTimeout(audioStopTimer)
      window.clearTimeout(ambientTimer)
      window.clearTimeout(drawWakeTimer)
      cancelAnimationFrame(animationFrame)
      audio?.pause()
    }
  })
</script>

<svelte:window onmousemove={movePointer} onmousedown={addClickWave} />

<div class="game-wave" class:active class:startup={mode === "startup"} aria-hidden={!active}>
  <canvas bind:this={canvas} {width} {height}></canvas>
  {#if mode === "startup"}
    <div class="game-wave-copy">
      <img
        class="game-wave-logo"
        src="./logo3.png"
        alt=""
        draggable="false"
      />
      <strong>마비노기 렘 부스터</strong>
    </div>
  {/if}
</div>
