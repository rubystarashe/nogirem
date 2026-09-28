// Canvas/audio implementation retained from GameWave; no Svelte runtime is used.
window.createNogiremWave = function(canvas, onEvent) {

  

  const onstartupcomplete = (...args) => onEvent("onstartupcomplete", args)
  const onstartuphidden = (...args) => onEvent("onstartuphidden", args)
  const onstartupidle = (...args) => onEvent("onstartupidle", args)
  const onplaybackstart = (...args) => onEvent("onplaybackstart", args)
  let logoRevealAnimation
  function onstartuplogomaskchange(active) {
    // The canvas and DOM logo must exchange visibility in this very paint,
    // without a round trip through Rust and the virtual DOM.
    document.documentElement.classList.toggle("nogirem-logo-mask", active)
    logoRevealAnimation?.cancel()
    logoRevealAnimation = null
    if (!active) {
      const logo = document.querySelector(".boost-home > img")
      if (logo) {
        const opacity = getComputedStyle(logo).opacity
        logoRevealAnimation = logo.animate([{opacity:0.7},{opacity}], {
          duration:600, easing:"ease", fill:"none",
        })
      }
    }
  }

  const width = 640
  const height = 290
  const timelineCueSeconds = 17.5
  const startupPlaybackCueSeconds = 18.5
  const track = [1800, 2100, 2350, 2700, 2850, 3900, 4150, 4400, 4700, 4950]
  
  let context
  let image
  let imageCache
  let imageReady = false
  let logoImage
  let logoImageReady = false
  let logoMaskCanvas
  let logoMaskContext
  let logoMaskSource
  let logoMaskFrame
  let startupLogoMaskActive = false
  let startupAllowed = false
  let startupStopPending = false
  let audio
  let animationFrame
  let drawWakeTimer
  let hideTimer
  let audioStopTimer
  let startupIdleTimer
  let playbackFallbackTimer
  let ambientTimer
  let active = true
  let mode = "startup"
  let startedAt = 0
  let playbackVolumeStartedAt = 0
  let circles = []
  let pointerX = 0
  let pointerY = 0
  let smoothX = 0
  let smoothY = 0
  let lastPointerFrame = 0
  let playbackId = 0
  let darkBackground = false
  let backgroundTransition = null
  let ambientEnabled = true
  let ambientBlockedUntil = 0
  let startupSequenceActive = false
  let renderedFrames = 0
  let pageVisible = true
  let startupMuted = false
  let logoVisible = true

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
    return performance.now() - startedAt
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

  function createLogoMaskCache() {
    const logoHeight = 150
    const logoWidth = Math.round(logoImage.width * (logoHeight / logoImage.height))
    const sourceCanvas = document.createElement("canvas")
    sourceCanvas.width = logoWidth
    sourceCanvas.height = logoHeight
    const sourceContext = sourceCanvas.getContext("2d")
    sourceContext.drawImage(logoImage, 0, 0, logoWidth, logoHeight)
    logoMaskSource = sourceContext.getImageData(0, 0, logoWidth, logoHeight)
    logoMaskCanvas = document.createElement("canvas")
    logoMaskCanvas.width = logoWidth
    logoMaskCanvas.height = logoHeight
    logoMaskContext = logoMaskCanvas.getContext("2d")
    logoMaskFrame = logoMaskContext.createImageData(logoWidth, logoHeight)
  }

  function drawStartupLogoMask(wave) {
    const logoWidth = logoMaskCanvas.width
    const logoHeight = logoMaskCanvas.height
    const logoX = Math.round((width - logoWidth) / 2)
    const logoY = 42
    const source = logoMaskSource.data
    const target = logoMaskFrame.data
    target.fill(0)
    for (let index = 0; index < source.length; index += 4) {
      const sourceAlpha = source[index + 3]
      if (sourceAlpha === 0) continue
      const pixel = index / 4
      const x = pixel % logoWidth
      const y = Math.floor(pixel / logoWidth)
      const distance = Math.hypot(
        logoX + x + 0.5 - wave.x,
        logoY + y + 0.5 - wave.y,
      )
      const insideWave = distance >= wave.innerRadius && distance <= wave.outerRadius
      const white = darkBackground ? !insideWave : insideWave
      const color = white ? 255 : 0
      target[index] = color
      target[index + 1] = color
      target[index + 2] = color
      target[index + 3] = Math.round(sourceAlpha * 0.7)
    }
    logoMaskContext.putImageData(logoMaskFrame, 0, 0)
    context.drawImage(logoMaskCanvas, logoX, logoY)
  }

  function requestDraw() {
    window.clearTimeout(drawWakeTimer)
    drawWakeTimer = null
    if (animationFrame || !context || !imageCache || (mode === "background" && !startupSequenceActive && !pageVisible)) return
    animationFrame = requestAnimationFrame(draw)
  }

  function scheduleDraw(delayMs) {
    if (animationFrame || drawWakeTimer || (mode === "background" && !startupSequenceActive && !pageVisible)) return
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
    const startupRemaining = Math.max(0, ambientBlockedUntil - performance.now())
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

  function draw(forceStatic = false) {
    animationFrame = null
    if (!context || !imageCache || (mode === "background" && !startupSequenceActive && !pageVisible && forceStatic !== true)) return
    renderedFrames += 1

    const now = performance.now()
    const elapsed = lastPointerFrame ? Math.max(0, now - lastPointerFrame) : 1000 / 60
    lastPointerFrame = now
    const follow = 1 - Math.pow(0.95, elapsed / (1000 / 60))
    smoothX += (pointerX - smoothX) * follow
    smoothY += (pointerY - smoothY) * follow
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
    let startupFinaleWave = null

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
      if (circle.kind === "startup-finale") {
        startupFinaleWave = {
          x: circle.x,
          y: circle.y,
          outerRadius: radius + stroke,
          innerRadius,
        }
      }
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

    const nextStartupLogoMaskActive = Boolean(startupFinaleWave && logoImageReady && logoVisible)
    if (nextStartupLogoMaskActive !== startupLogoMaskActive) {
      startupLogoMaskActive = nextStartupLogoMaskActive
      onstartuplogomaskchange(startupLogoMaskActive)
    }
    if (startupLogoMaskActive) {
      drawStartupLogoMask(startupFinaleWave)
    }

    if (forceStatic === true) return
    const audioPlaying = Boolean(audio && !audio.paused)
    if (audioPlaying) {
      const fadeOut = track.at(-1) - timelineElapsed + 1000
      const fadeIn = (performance.now() - playbackVolumeStartedAt) / 2000
      audio.volume = Math.max(0, Math.min(1, fadeIn, fadeOut / 2000)) * 0.2
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
    const bounds = canvas.getBoundingClientRect()
    if (!bounds.width || !bounds.height) return
    if (!animationFrame) lastPointerFrame = performance.now()
    pointerX = ((event.clientX - bounds.left) / bounds.width) * width - width / 2
    pointerY = ((event.clientY - bounds.top) / bounds.height) * height - height / 2
    requestDraw()
  }

  function addClickWave(event) {
    if (
      event.target instanceof Element
      && event.target.closest(
        ".boost-text-area, .optimization-summary, .character-guide-link, .dxvk-update-link, .creator-credit, .creator-view",
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
        time: performance.now() - startedAt,
      },
    ]
    requestDraw()
  }

  function makeActionWave(clientX, clientY, paused = false) {
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
        time: performance.now() - startedAt,
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
      startedAt: performance.now() + transitionDelay,
      duration,
    }
    if (paused) stopAmbientWaves()
    else if (ambientEnabled) scheduleAmbientWaves(1500)
    requestDraw()
    return { duration, radius: size, delay: transitionDelay }
  }

  function setPaused(paused) {
    // Called after the UI transition finishes. Settle its final color even if
    // background RAF is suspended; unchanged settings are filtered in Rust.
    if (darkBackground === paused && !backgroundTransition) return
    darkBackground = paused
    backgroundTransition = null
    if (paused) stopAmbientWaves()
    else if (ambientEnabled) scheduleAmbientWaves()
    requestDraw()
  }

  function setAmbientEnabled(enabled) {
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

  function setPageVisible(visible) {
    const nextVisible = Boolean(visible)
    if (pageVisible === nextVisible) return
    pageVisible = nextVisible
    if (!pageVisible) {
      window.clearTimeout(ambientTimer)
      ambientTimer = null
      // Startup keeps its audio/identity timeline. Background visuals sleep
      // until the app is active again, including pending waves and pointer motion.
      if (mode === "background" && !startupSequenceActive) {
        cancelAnimationFrame(animationFrame)
        animationFrame = null
        window.clearTimeout(drawWakeTimer)
        drawWakeTimer = null
        circles = []
        if (backgroundTransition) {
          darkBackground = backgroundTransition.targetDark
          backgroundTransition = null
        }
        draw(true)
      }
      return
    }
    requestDraw()
    if (
      ambientEnabled
      && mode === "background"
      && (!darkBackground || backgroundTransition?.targetDark === false)
    ) {
      const timelineElapsed = currentTimelineElapsed()
      const ambientRemaining = Math.max(
        0,
        ...circles
          .filter(circle => circle.kind === "ambient")
          .map(circle => circle.time + (circle.duration ?? circle.size * 8) - timelineElapsed),
      )
      if (ambientRemaining > 0) {
        ambientTimer = window.setTimeout(
          () => scheduleAmbientWaves(300),
          ambientRemaining,
        )
      } else {
        scheduleAmbientWaves(300)
      }
    }
  }

  function startPlayback(currentPlaybackId) {
    const nextAudio = audio
    nextAudio.volume = 0
    let animationStarted = false

    const startAnimation = () => {
      if (animationStarted || currentPlaybackId !== playbackId) return
      animationStarted = true
      window.clearTimeout(playbackFallbackTimer)
      playbackFallbackTimer = null
      circles = createCircles()
      const initialTimelineElapsed = startupMuted
        ? (startupPlaybackCueSeconds - timelineCueSeconds) * 1000
        : Math.max(0, (nextAudio.currentTime - timelineCueSeconds) * 1000)
      startedAt = performance.now() - initialTimelineElapsed
      playbackVolumeStartedAt = performance.now()
      const finalStartupWaveEnd = Math.max(
        6200,
        ...circles.map(circle => circle.time + (circle.duration ?? circle.size * 8)),
      )
      ambientBlockedUntil = startedAt + finalStartupWaveEnd
      cancelAnimationFrame(animationFrame)
      animationFrame = null
      requestDraw()
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          if (currentPlaybackId === playbackId) onplaybackstart()
        })
      })
      hideTimer = window.setTimeout(() => {
        onstartupcomplete()
      }, Math.max(0, 2500 - initialTimelineElapsed))
      audioStopTimer = window.setTimeout(() => {
        nextAudio.pause()
      }, Math.max(0, 6200 - initialTimelineElapsed))
      const finishStartupSequence = () => {
        const remaining = finalStartupWaveEnd - currentTimelineElapsed()
        if (remaining > 0) {
          startupIdleTimer = window.setTimeout(finishStartupSequence, Math.ceil(remaining))
          return
        }
        // The introductory overlay disappears before the finale wave ends.
        // Complete its final paint even if focus was lost during startup.
        cancelAnimationFrame(animationFrame)
        animationFrame = null
        draw()
        startupSequenceActive = false
        if (!pageVisible) {
          cancelAnimationFrame(animationFrame)
          animationFrame = null
          window.clearTimeout(drawWakeTimer)
          drawWakeTimer = null
        }
        scheduleAmbientWaves(0)
        onstartupidle()
      }
      startupIdleTimer = window.setTimeout(finishStartupSequence, Math.max(0, Math.ceil(finalStartupWaveEnd - initialTimelineElapsed)))
    }

    if (startupMuted) {
      startAnimation()
      return
    }

    const playFromCue = () => {
      if (animationStarted || currentPlaybackId !== playbackId) return
      nextAudio.currentTime = startupPlaybackCueSeconds
      nextAudio.addEventListener("playing", startAnimation, { once: true })
      void nextAudio.play().catch(error => {
        console.warn("게임 시작 음악을 재생하지 못했습니다", error)
        startAnimation()
      })
    }

    playbackFallbackTimer = window.setTimeout(() => {
      console.warn("시작 음악 준비가 지연되어 무음으로 시작합니다")
      startAnimation()
    }, 1500)
    nextAudio.addEventListener("error", startAnimation, { once: true })
    if (nextAudio.readyState >= HTMLMediaElement.HAVE_METADATA) playFromCue()
    else nextAudio.addEventListener("loadedmetadata", playFromCue, { once: true })
  }

  function setStartupMuted(muted) {
    startupMuted = Boolean(muted)
    if (startupMuted) audio?.pause()
  }

  function playStartup() {
    startupSequenceActive = true
    playbackId += 1
    const currentPlaybackId = playbackId
    window.clearTimeout(hideTimer)
    window.clearTimeout(audioStopTimer)
    window.clearTimeout(startupIdleTimer)
    window.clearTimeout(playbackFallbackTimer)
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

  function allowStartup() {
    if (startupAllowed) return
    startupAllowed = true
    if (imageReady && logoImageReady) playStartup()
  }

  function skipStartup() {
    if (mode !== "startup") return
    playbackId += 1
    startupAllowed = false
    startupSequenceActive = false
    startupStopPending = false
    window.clearTimeout(hideTimer)
    window.clearTimeout(audioStopTimer)
    window.clearTimeout(startupIdleTimer)
    window.clearTimeout(playbackFallbackTimer)
    window.clearTimeout(drawWakeTimer)
    cancelAnimationFrame(animationFrame)
    animationFrame = null
    drawWakeTimer = null
    audio?.pause()
    circles = []
    ambientBlockedUntil = 0
    active = true
    mode = "background"
    if (startupLogoMaskActive) {
      startupLogoMaskActive = false
      onstartuplogomaskchange(false)
    }
    onstartuphidden()
    onstartupidle()
    scheduleAmbientWaves()
    requestDraw()
  }

  function finishStartup() {
    if (mode !== "startup") return
    if (!imageReady) startupStopPending = true
    window.clearTimeout(hideTimer)
    hideTimer = window.setTimeout(() => {
      active = true
      mode = "background"
      onstartuphidden()
      scheduleAmbientWaves()
      requestDraw()
    }, 450)
  }

  const cleanup = (() => {
    context = canvas.getContext("2d")
    audio = new Audio("/public/ost.mp3")
    audio.preload = "auto"
    audio.load()
    image = new Image()
    image.onload = () => {
      createImageCache()
      imageReady = true
      if (startupStopPending) finishStartup()
      else if (startupAllowed && logoImageReady) playStartup()
      else if (mode === "background") requestDraw()
    }
    image.onerror = error => {
      console.warn("시작 배경 이미지를 불러오지 못했습니다", error)
      imageReady = true
      if (startupStopPending) finishStartup()
      else if (startupAllowed && logoImageReady) playStartup()
    }
    image.src = "/public/main3-optimized.jpg"
    logoImage = new Image()
    logoImage.onload = () => {
      createLogoMaskCache()
      logoImageReady = true
      if (startupAllowed && imageReady) playStartup()
      requestDraw()
    }
    logoImage.onerror = error => {
      console.warn("시작 로고 이미지를 불러오지 못했습니다", error)
      logoImageReady = true
      if (startupAllowed && imageReady) playStartup()
    }
    logoImage.src = "/public/logo3.png"
    return () => {
      playbackId += 1
      window.clearTimeout(hideTimer)
      window.clearTimeout(audioStopTimer)
      window.clearTimeout(startupIdleTimer)
      window.clearTimeout(playbackFallbackTimer)
      window.clearTimeout(ambientTimer)
      window.clearTimeout(drawWakeTimer)
      cancelAnimationFrame(animationFrame)
      audio?.pause()
      if (startupLogoMaskActive) onstartuplogomaskchange(false)
      logoRevealAnimation?.cancel()
      document.documentElement.classList.remove("nogirem-logo-mask")
    }
  })()

window.addEventListener("mousemove", movePointer, { capture: true, passive: true });
window.addEventListener("mousedown", addClickWave);
return { setLogoVisible(value) { if (logoVisible !== value) { logoVisible = value; requestDraw(); } }, inspect: () => ({startupSequenceActive, renderedFrames, pageVisible, framePending: Boolean(animationFrame), ambientPending: Boolean(ambientTimer), ambientCircles: circles.filter(circle => circle.kind === "ambient").length, darkBackground, logoMask: startupLogoMaskActive, pointerX, pointerY, offsetX: smoothX / 20, offsetY: smoothY / 10, mode, timeline: currentTimelineElapsed(), circles: circles.length, audioPaused: audio.paused, audioTime: audio.currentTime, audioVolume: audio.volume, audioError: audio.error?.message, transition: backgroundTransition}), makeActionWave, setPaused, setAmbientEnabled, setPageVisible, setStartupMuted, allowStartup, skipStartup, finishStartup, dispose() { cleanup(); window.removeEventListener("mousemove", movePointer, true); window.removeEventListener("mousedown", addClickWave); } };
};
