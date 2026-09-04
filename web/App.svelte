<script>
  import { onMount } from "svelte"
  import packageInfo from "../package.json"
  import GameWave from "./GameWave.svelte"
  import Modal from "./Modal.svelte"
  import NoticeModal from "./NoticeModal.svelte"

  let services = {
    nvidia: { loading: true, data: null, error: null },
    network: { loading: true, data: null, error: null },
    affinity: { loading: true, data: null, error: null },
    memory: { loading: true, data: null, error: null },
  }
  let refreshing = false
  let includeNic = false
  let affinityRuntimeSyncing = false
  let memoryRuntimeSyncing = false
  let frameBoostAction = null
  let closeModalVisible = false
  let optimizationModalVisible = false
  let conflictModalVisible = false
  let networkReconnectModalVisible = false
  let networkReconnectCloseSignal = 0
  let networkReconnectAction = null
  let conflictPrograms = []
  let ignoredConflictSignature = ""
  let conflictModalCloseSignal = 0
  let closeActionPending = false
  let gameWave
  let settingsVisible = false
  let startupDataReady = false
  let startupAnimationFinished = false
  let startupLogoMaskActive = false
  let interfaceVisible = false
  let documentVisible = true
  let windowVisuallyActive = true
  let pageVisible = true
  let spinnerAnimationVisible = true
  let spinnerFinishTimer
  let statusTransitionPhase = "enter"
  let statusTransitionFrom = "노기렘 실행중"
  let statusTransitionTo = "실시간 부스트중"
  let statusTransitionId = 0
  let displayedStatusText = "실시간 부스트중"
  let visualPaused = false
  let colorTransition = null
  let colorTransitionId = 0
  let displayedGuideText = "마비노기를 위해 모든 프로세스를 최적화 하고 있습니다"
  let guideTransitionFrom = displayedGuideText
  let guideTransitionTo = displayedGuideText
  let guideTransitionPhase = "done"
  let guideTransitionId = 0
  let startupIdentityPhase = "brand"
  let startupIdentityTimer
  let leftTopContentTimer
  let leftTopContentEntered = false
  let testVersionNoticeVisible = false
  let testVersionNoticeScheduled = false
  let testVersionNoticeTimer

  const versionText = "미공개 테스트 버전"
  const ambientRhythmEnabled = true
  const boostSpinnerEnabled = true

  const goalNames = {
    verticalSyncOff: "수직 동기화 끄기",
    maxFrameRate400: "최대 프레임 상한 설정",
    threadedOptimizationOn: "스레드 최적화",
    preferMaximumPerformance: "최고 성능 선호",
    ultraLowLatency: "저지연 모드 울트라",
  }

  function messageOf(error) {
    return error?.message?.replace(/^Error invoking remote method '[^']+': Error: /, "")
      ?? String(error)
  }

  function updateService(key, values) {
    services = {
      ...services,
      [key]: { ...services[key], ...values },
    }
  }

  function receiveResult(key, result) {
    updateService(key, result.ok
      ? { loading: false, data: result.data, error: null }
      : { loading: false, data: null, error: result.error?.message ?? "상태 확인 실패" })
  }

  function finishStartupWhenReady() {
    if (startupDataReady && startupAnimationFinished) gameWave?.finishStartup()
  }

  function handleStartupComplete() {
    startupAnimationFinished = true
    finishStartupWhenReady()
  }

  function handleStartupPlaybackStart() {
    void window.nogirem.beginStartupReveal()
  }

  function openDxvkWindow() {
    if (services.affinity.data?.renderer?.mode === "direct3d9") {
      void window.nogirem.openDxvkGuide()
      return
    }
    void window.nogirem.openDxvkManager()
  }

  function dxvkLinkState() {
    if (services.affinity.data?.renderer?.mode === "direct3d9") return "not-in-use"
    if (services.affinity.data?.dxvk?.state === "latest") return "latest"
    if (services.affinity.data?.dxvk?.state === "update-required") return "update-required"
    return "checking"
  }

  function applyVisualActivity() {
    const nextPageVisible = documentVisible && windowVisuallyActive
    if (nextPageVisible) {
      window.clearTimeout(spinnerFinishTimer)
      spinnerFinishTimer = null
      spinnerAnimationVisible = true
    } else if (pageVisible) {
      window.clearTimeout(spinnerFinishTimer)
      spinnerFinishTimer = window.setTimeout(() => {
        spinnerAnimationVisible = false
        spinnerFinishTimer = null
      }, 1500)
    }
    pageVisible = nextPageVisible
    gameWave?.setPageVisible(pageVisible)
  }

  function syncPageVisibility() {
    documentVisible = document.visibilityState === "visible"
    applyVisualActivity()
  }

  function setWindowVisualActivity(active) {
    windowVisuallyActive = active
    applyVisualActivity()
  }

  function handleStartupHidden() {
    const statusText = frameBoostStatusText()
    visualPaused = isPausedStatus(statusText)
    gameWave?.setPaused(visualPaused)
    gameWave?.setAmbientEnabled(ambientRhythmEnabled && statusText === "실시간 부스트중")
    displayedStatusText = statusText
    displayedGuideText = guideTextForStatus(statusText)
    statusTransitionPhase = "done"
    startupIdentityPhase = "brand"
    leftTopContentEntered = false
    interfaceVisible = true
    syncConflictWarning(
      services.affinity.data?.conflictingPrograms,
      statusText === "실시간 부스트중",
    )
    window.clearTimeout(startupIdentityTimer)
    startupIdentityTimer = window.setTimeout(() => {
      startupIdentityPhase = "transition"
    }, 250)
  }

  function finishStartupIdentityTransition() {
    if (startupIdentityPhase !== "transition") return
    startupIdentityPhase = "version"
    window.clearTimeout(startupIdentityTimer)
    startupIdentityTimer = window.setTimeout(() => {
      const statusText = frameBoostStatusText()
      displayedStatusText = statusText
      displayedGuideText = guideTextForStatus(statusText)
      startupIdentityPhase = "done"
      window.clearTimeout(leftTopContentTimer)
      leftTopContentTimer = window.setTimeout(() => {
        leftTopContentEntered = true
      }, 2030)
      animateStatusTransition(versionText, statusText)
    }, 1500)
  }

  function startColorTransition(clientX, clientY, paused, duration, radius, delay = 0) {
    colorTransitionId += 1
    visualPaused = !paused
    colorTransition = {
      id: colorTransitionId,
      clientX,
      clientY,
      paused,
      duration,
      radius,
      delay,
    }
  }

  function finishColorTransition(event, id) {
    if (event.target !== event.currentTarget) return
    if (colorTransition?.id !== id) return
    visualPaused = colorTransition.paused
    colorTransition = null
  }

  function frameBoostStatusText() {
    if (!(services.affinity.data?.running && services.memory.data?.running)) {
      return "실시간 적용 일시정지됨"
    }
    return services.affinity.data?.gameActive || services.memory.data?.gameActive
      ? "실시간 부스트중"
      : "부스트 대기중"
  }

  function isPausedStatus(statusText) {
    return statusText === "실시간 적용 일시정지됨"
  }

  function guideTextForStatus(statusText) {
    if (statusText === "실시간 부스트중") {
      return "마비노기를 위해 모든 프로세스를 최적화 하고 있습니다"
    }
    if (statusText === "부스트 대기중") return "마비노기 클라이언트를 찾고 있습니다"
    return "적용된 부스트 설정은 여전히 남아있습니다"
  }

  function syncConflictWarning(programs = [], boostActive = false) {
    const detected = [...new Set(programs)].sort()
    if (!boostActive || detected.length === 0) {
      conflictModalVisible = false
      if (detected.length === 0) ignoredConflictSignature = ""
      return
    }
    const signature = detected.join("|")
    conflictPrograms = detected
    if (interfaceVisible && signature !== ignoredConflictSignature) {
      optimizationModalVisible = false
      conflictModalVisible = true
    }
  }

  function dismissConflictWarning() {
    ignoredConflictSignature = conflictPrograms.join("|")
    conflictModalVisible = false
  }

  function syncDisplayedBoostStatus() {
    if (!interfaceVisible || frameBoostAction) return
    const nextStatusText = frameBoostStatusText()
    gameWave?.setAmbientEnabled(ambientRhythmEnabled && nextStatusText === "실시간 부스트중")
    if (startupIdentityPhase !== "done") {
      displayedStatusText = nextStatusText
      displayedGuideText = guideTextForStatus(nextStatusText)
      return
    }
    if (nextStatusText !== displayedStatusText) {
      animateStatusTransition(displayedStatusText, nextStatusText)
    }
  }

  function animateStatusTransition(from, to) {
    displayedStatusText = to
    const nextGuideText = guideTextForStatus(to)
    if (displayedGuideText !== nextGuideText) {
      guideTransitionFrom = displayedGuideText
      guideTransitionTo = nextGuideText
      displayedGuideText = nextGuideText
      guideTransitionPhase = "out"
      guideTransitionId += 1
    }
    if (from === to) return
    statusTransitionFrom = from
    statusTransitionTo = to
    statusTransitionPhase = "enter"
    statusTransitionId += 1
  }

  function finishStatusTransition() {
    statusTransitionPhase = "done"
    if (testVersionNoticeScheduled) return
    testVersionNoticeScheduled = true
    window.clearTimeout(testVersionNoticeTimer)
    testVersionNoticeTimer = window.setTimeout(() => {
      testVersionNoticeVisible = true
    }, 3000)
  }

  async function loadAll() {
    refreshing = true
    services = {
      nvidia: { ...services.nvidia, loading: true, error: null },
      network: { ...services.network, loading: true, error: null },
      affinity: { ...services.affinity, loading: true, error: null },
      memory: { ...services.memory, loading: true, error: null },
    }
    try {
      const result = await window.nogirem.getStatus()
      receiveResult("nvidia", result.nvidia)
      receiveResult("network", result.network)
      receiveResult("affinity", result.affinity)
      receiveResult("memory", result.memory)
      if (result.affinity.ok && result.affinity.data.running) {
        includeNic = result.affinity.data.includeNic
      }
    } catch (error) {
      const message = messageOf(error)
      updateService("nvidia", { loading: false, error: message })
      updateService("network", { loading: false, error: message })
      updateService("affinity", { loading: false, error: message })
      updateService("memory", { loading: false, error: message })
    } finally {
      refreshing = false
    }
  }

  async function refreshService(key, operation) {
    updateService(key, { loading: true, error: null })
    try {
      const data = await operation()
      updateService(key, { loading: false, data, error: null })
    } catch (error) {
      updateService(key, { loading: false, error: messageOf(error) })
    }
  }

  async function optimize(key, operation) {
    updateService(key, { optimizing: true, error: null })
    try {
      const data = await operation()
      updateService(key, { optimizing: false, loading: false, data, error: null })
    } catch (error) {
      updateService(key, { optimizing: false, error: messageOf(error) })
    }
  }

  function fastPingReconnectRequired() {
    const current = services.network.data?.fastPing?.current
    return current?.TcpAckFrequency !== 1 || current?.TCPNoDelay !== 1
  }

  function requestNetworkOptimization() {
    if (!fastPingReconnectRequired()) {
      void optimize("network", window.nogirem.optimizeNetwork)
      return
    }
    networkReconnectAction = null
    networkReconnectModalVisible = true
  }

  function closeNetworkReconnectModal(action) {
    if (networkReconnectAction) return
    networkReconnectAction = action
    networkReconnectCloseSignal += 1
  }

  function finishNetworkReconnectModal() {
    const action = networkReconnectAction
    networkReconnectModalVisible = false
    networkReconnectAction = null
    if (action === "continue") {
      void optimize("network", window.nogirem.optimizeNetwork)
    }
  }

  function nvidiaReady(data) {
    return data?.supported && data?.nvidia && data?.goals?.allMet
  }

  function networkReady(data) {
    return data?.optimized
  }

  function formatMiB(bytes) {
    return `${Math.round((bytes ?? 0) / 1024 / 1024).toLocaleString()} MiB`
  }

  function toggleFrameBoostFromStatus(event) {
    if (
      frameBoostAction
      || colorTransition
      || displayedStatusText === "부스트 대기중"
    ) return
    const enabled = !(services.affinity.data?.running || services.memory.data?.running)
    const paused = !enabled
    const wave = gameWave?.makeActionWave(event.clientX, event.clientY, paused)
      ?? { duration: paused ? 600 : 3000, radius: paused ? 420 : 600, delay: paused ? 0 : 500 }
    gameWave?.setAmbientEnabled(
      ambientRhythmEnabled && enabled && frameBoostStatusText() === "실시간 부스트중",
    )
    startColorTransition(
      event.clientX,
      event.clientY,
      paused,
      wave.duration,
      wave.radius,
      wave.delay,
    )
    void toggleFrameBoost(true)
  }

  async function toggleFrameBoost(waveStarted = false) {
    if (frameBoostAction || (colorTransition && waveStarted !== true)) return
    const enabled = !(services.affinity.data?.running || services.memory.data?.running)
    const previousStatusText = displayedStatusText
    const requestedStatusText = enabled
      ? (services.affinity.data?.gameActive || services.memory.data?.gameActive
        ? "실시간 부스트중"
        : "부스트 대기중")
      : "실시간 적용 일시정지됨"
    if (waveStarted !== true) {
      visualPaused = !enabled
      colorTransition = null
      gameWave?.setPaused(!enabled)
    }
    animateStatusTransition(previousStatusText, requestedStatusText)
    frameBoostAction = enabled ? "run" : "pause"
    updateService("affinity", { optimizing: true, error: null })
    updateService("memory", { optimizing: true, error: null })
    try {
      const result = await window.nogirem.setFrameBoostEnabled({ enabled, includeNic })
      updateService("affinity", {
        optimizing: false,
        loading: false,
        data: result.affinity,
        error: null,
      })
      updateService("memory", {
        optimizing: false,
        loading: false,
        data: result.memory,
        error: null,
      })
      if (result.affinity.running) includeNic = result.affinity.includeNic
      const actualStatusText = frameBoostStatusText()
      gameWave?.setAmbientEnabled(
        ambientRhythmEnabled && actualStatusText === "실시간 부스트중",
      )
      syncConflictWarning(
        result.affinity.conflictingPrograms,
        actualStatusText === "실시간 부스트중",
      )
      if (actualStatusText !== requestedStatusText) {
        const actualPaused = isPausedStatus(actualStatusText)
        if (actualPaused !== isPausedStatus(requestedStatusText)) {
          visualPaused = actualPaused
          colorTransition = null
          gameWave?.setPaused(actualPaused)
        }
        animateStatusTransition(requestedStatusText, actualStatusText)
      }
    } catch (error) {
      const message = messageOf(error)
      updateService("affinity", { optimizing: false, error: message })
      updateService("memory", { optimizing: false, error: message })
      visualPaused = isPausedStatus(previousStatusText)
      colorTransition = null
      gameWave?.setPaused(visualPaused)
      gameWave?.setAmbientEnabled(
        ambientRhythmEnabled && previousStatusText === "실시간 부스트중",
      )
      animateStatusTransition(requestedStatusText, previousStatusText)
    } finally {
      frameBoostAction = null
    }
  }

  async function resetFrameBoost() {
    frameBoostAction = "reset"
    updateService("affinity", { optimizing: true, error: null })
    updateService("memory", { optimizing: true, error: null })
    try {
      const result = await window.nogirem.resetFrameBoost()
      updateService("affinity", {
        optimizing: false,
        loading: false,
        data: result.affinity,
        error: null,
      })
      updateService("memory", {
        optimizing: false,
        loading: false,
        data: result.memory,
        error: null,
      })
    } catch (error) {
      const message = messageOf(error)
      updateService("affinity", { optimizing: false, error: message })
      updateService("memory", { optimizing: false, error: message })
    } finally {
      frameBoostAction = null
    }
  }

  async function confirmClose(action) {
    if (closeActionPending) return
    if (action === "cancel") closeModalVisible = false
    else closeActionPending = true
    try {
      await window.nogirem.confirmClose(action)
    } catch (error) {
      if (action !== "cancel") {
        closeActionPending = false
        closeModalVisible = true
        updateService("affinity", { error: messageOf(error) })
      }
    }
  }

  async function requestApplicationClose() {
    if (closeActionPending) return
    closeActionPending = true
    try {
      await window.nogirem.requestClose()
    } catch (error) {
      closeActionPending = false
      updateService("affinity", { error: messageOf(error) })
    }
  }

  async function syncAffinityRuntime() {
    if (
      closeActionPending
      || affinityRuntimeSyncing
      || services.affinity.optimizing
      || !services.affinity.data?.running
      || document.visibilityState !== "visible"
    ) {
      return
    }

    affinityRuntimeSyncing = true
    try {
      const runtime = await window.nogirem.getAffinityRuntime()
      if (closeActionPending) return
      const current = services.affinity.data
      updateService("affinity", {
        data: {
          ...current,
          running: runtime.running,
          gameActive: runtime.gameActive,
          includeNic: runtime.includeNic,
          nicManaged: runtime.nicManaged,
          renderer: runtime.renderer,
          characterSimplification: runtime.characterSimplification,
          dxvk: runtime.dxvk,
          conflictingPrograms: runtime.conflictingPrograms,
        },
      })
      if (runtime.running) includeNic = runtime.includeNic
      syncDisplayedBoostStatus()
      syncConflictWarning(
        runtime.conflictingPrograms,
        frameBoostStatusText() === "실시간 부스트중",
      )
    } catch {
    } finally {
      affinityRuntimeSyncing = false
    }
  }

  async function syncMemoryRuntime() {
    if (
      closeActionPending
      || memoryRuntimeSyncing
      || services.memory.optimizing
      || !services.memory.data?.running
      || document.visibilityState !== "visible"
    ) {
      return
    }

    memoryRuntimeSyncing = true
    try {
      const runtime = await window.nogirem.getMemoryRuntime()
      if (closeActionPending) return
      updateService("memory", {
        data: {
          ...services.memory.data,
          ...runtime,
        },
      })
      syncDisplayedBoostStatus()
    } catch {
    } finally {
      memoryRuntimeSyncing = false
    }
  }

  onMount(() => {
    document.addEventListener("visibilitychange", syncPageVisibility)
    const removeVisualActivityListener = window.nogirem.onVisualActivityChanged(
      setWindowVisualActivity,
    )
    syncPageVisibility()
    void window.nogirem.getVisualActivity().then(setWindowVisualActivity)
    void loadAll().finally(() => {
      startupDataReady = true
      gameWave?.allowStartup()
      finishStartupWhenReady()
    })
    const removeCloseListener = window.nogirem.onCloseRequested(() => {
      closeActionPending = false
      closeModalVisible = true
    })
    const timer = window.setInterval(() => {
      void syncAffinityRuntime()
      void syncMemoryRuntime()
    }, 500)
    return () => {
      window.clearInterval(timer)
      window.clearTimeout(startupIdentityTimer)
      window.clearTimeout(leftTopContentTimer)
      window.clearTimeout(testVersionNoticeTimer)
      window.clearTimeout(spinnerFinishTimer)
      document.removeEventListener("visibilitychange", syncPageVisibility)
      removeVisualActivityListener()
      removeCloseListener()
    }
  })
</script>

<svelte:head>
  <title>마비노기 렘 부스터 - {packageInfo.version}</title>
</svelte:head>

<div class="window-drag" aria-hidden="true"></div>
<div class="window-controls">
  <button
    class="window-control window-close"
    aria-label="프로그램 닫기"
    onclick={requestApplicationClose}
  >
    <svg class="control-icon-bg" width="25" height="25" aria-hidden="true">
      <line x1="0" y1="0" x2="25" y2="25" stroke-width="2" />
      <line x1="25" y1="0" x2="0" y2="25" stroke-width="2" />
    </svg>
    <svg width="25" height="25" aria-hidden="true">
      <line class="control-line-one" x1="0" y1="0" x2="25" y2="25" stroke-width="2" />
      <line class="control-line-two" x1="25" y1="0" x2="0" y2="25" stroke-width="2" />
    </svg>
  </button>
</div>

<span class="creator-credit entered">
  [류트@렘] 제작
</span>

<main class="compact-shell">
  {#if interfaceVisible}
    {#if settingsVisible}
    <section class="settings-panel" aria-labelledby="settings-title">
      <div class="settings-heading">
        <button class="settings-back" aria-label="돌아가기" onclick={() => settingsVisible = false}>
          ←
        </button>
        <h1 id="settings-title">설정</h1>
      </div>

      <div class="setting-row">
        <div>
          <strong>실시간 부스트</strong>
          <span>
            {services.affinity.data?.running && services.memory.data?.running
              ? "실행 중"
              : "일시정지"}
          </span>
        </div>
        <label class="compact-check">
          <input
            type="checkbox"
            bind:checked={includeNic}
            disabled={services.affinity.data?.running || services.affinity.optimizing}
          />
          NIC RSS
        </label>
        <button
          class="setting-action"
          disabled={services.affinity.optimizing
            || services.memory.optimizing
            || frameBoostAction
            || colorTransition}
          onclick={toggleFrameBoost}
        >
          {services.affinity.data?.running || services.memory.data?.running ? "일시정지" : "실행"}
        </button>
      </div>

      <div class="setting-row">
        <div>
          <strong>네트워크 최적화</strong>
          <span>{networkReady(services.network.data) ? "최적화됨" : "확인 필요"}</span>
        </div>
        <button
          class="setting-action"
          disabled={services.network.loading
            || services.network.optimizing
            || networkReady(services.network.data)}
          onclick={requestNetworkOptimization}
        >
          {services.network.optimizing ? "적용 중" : "최적화"}
        </button>
      </div>

      <div class="setting-row">
        <div>
          <strong>NVIDIA 최적화</strong>
          <span>{nvidiaReady(services.nvidia.data) ? "최적화됨" : "확인 필요"}</span>
        </div>
        <button
          class="setting-action"
          disabled={services.nvidia.loading
            || services.nvidia.optimizing
            || !services.nvidia.data?.nvidia
            || nvidiaReady(services.nvidia.data)}
          onclick={() => optimize("nvidia", window.nogirem.optimizeNvidia)}
        >
          {services.nvidia.optimizing ? "적용 중" : "최적화"}
        </button>
      </div>
    </section>
    {:else}
      <section
        class="boost-home"
        class:paused={visualPaused}
        aria-label="실시간 부스트 상태"
      >
        <img
          class:canvas-logo-hidden={startupLogoMaskActive}
          src="./logo3.png"
          alt=""
          draggable="false"
        />
        {#if startupIdentityPhase === "done"}
          <button
            class="optimization-summary"
            class:entered={leftTopContentEntered}
            aria-label="최적화 상세 상태 열기"
            onclick={() => optimizationModalVisible = true}
          >
            <div class:ready={nvidiaReady(services.nvidia.data)}>
              {#if nvidiaReady(services.nvidia.data)}
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9Z" />
                </svg>
              {:else}
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M1 21h22L12 2 1 21Zm12-3h-2v-2h2v2Zm0-4h-2v-4h2v4Z" />
                </svg>
              {/if}
              <span>
                그래픽 설정
                {services.nvidia.loading
                  ? "확인 중"
                  : (nvidiaReady(services.nvidia.data) ? "최적화됨" : "확인 필요")}
              </span>
            </div>
            <div class:ready={networkReady(services.network.data)}>
              {#if networkReady(services.network.data)}
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9Z" />
                </svg>
              {:else}
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M1 21h22L12 2 1 21Zm12-3h-2v-2h2v2Zm0-4h-2v-4h2v4Z" />
                </svg>
              {/if}
              <span>
                네트워크
                {services.network.loading
                  ? "확인 중"
                  : (networkReady(services.network.data) ? "최적화됨" : "확인 필요")}
              </span>
            </div>
          </button>
          <button
            class="character-guide-link"
            class:ready={services.affinity.data?.characterSimplification?.applied}
            class:warning={!services.affinity.data?.characterSimplification?.applied}
            class:entered={leftTopContentEntered}
            onclick={() => window.nogirem.openCharacterGuide()}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              {#if services.affinity.data?.characterSimplification?.applied}
                <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9Z" />
              {:else}
                <path d="M1 21h22L12 2 1 21Zm12-3h-2v2h2v-2Zm0-2h-2v-4h2v4Z" />
              {/if}
            </svg>
            <span>주변 캐릭터 강제 간소화</span>
          </button>
          <button
            class="dxvk-update-link"
            class:ready={dxvkLinkState() === "latest"}
            class:warning={["not-in-use", "update-required"].includes(dxvkLinkState())}
            class:entered={leftTopContentEntered}
            onclick={openDxvkWindow}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              {#if ["not-in-use", "update-required"].includes(dxvkLinkState())}
                <path d="M1 21h22L12 2 1 21Zm12-3h-2v2h2v-2Zm0-2h-2v-4h2v4Z" />
              {:else if dxvkLinkState() === "latest"}
                <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9Z" />
              {:else}
                <path d="M12 4V1L8 5l4 4V6a6 6 0 0 1 5.65 8H19.7A8 8 0 0 0 12 4Zm-5.65 6H4.3A8 8 0 0 0 12 20v3l4-4-4-4v3a6 6 0 0 1-5.65-8Z" />
              {/if}
            </svg>
            <span>
              {dxvkLinkState() === "not-in-use"
                ? "Vulkan을 사용중이지 않음"
                : dxvkLinkState() === "update-required"
                  ? "Vulkan 업데이트가 필요함"
                  : dxvkLinkState() === "latest"
                    ? "Vulkan 최신버전 사용중"
                    : "DXVK 업데이트"}
            </span>
          </button>
        {/if}
        <button
          class="boost-text-area"
          class:paused-target={isPausedStatus(statusTransitionTo)}
          aria-label="실시간 부스트 실행 상태 전환"
          disabled={startupIdentityPhase !== "done"
            || displayedStatusText === "부스트 대기중"
            || frameBoostAction
            || colorTransition}
          onclick={toggleFrameBoostFromStatus}
        >
          {#if startupIdentityPhase === "brand"}
            <span class="startup-identity-text">마비노기 렘 부스터</span>
          {:else if startupIdentityPhase === "transition"}
            <span class="startup-identity-text identity-roulette-out">마비노기 렘 부스터</span>
            <span
              class="startup-identity-text identity-roulette-in"
              onanimationend={finishStartupIdentityTransition}
            >
              {versionText}
            </span>
          {:else if startupIdentityPhase === "version"}
            <span class="startup-identity-text">{versionText}</span>
          {:else}
            <span
              class="boost-text-final"
              class:concealed={statusTransitionPhase === "enter"}
              class:boosting={displayedStatusText === "실시간 부스트중"
                && statusTransitionPhase === "done"}
              class:animation-paused={!pageVisible}
            >
              {displayedStatusText}
            </span>
            {#key statusTransitionId}
              {#if statusTransitionPhase === "enter"}
                <span
                  class="boost-text-base transitioning"
                  class:paused-source={statusTransitionFrom === "실시간 적용 일시정지됨"}
                >
                  {statusTransitionFrom}
                </span>
                <span
                  class="boost-text-over"
                  onanimationend={() => statusTransitionPhase = "leave"}
                >
                  {statusTransitionTo}
                </span>
              {:else if statusTransitionPhase === "leave"}
                <span
                  class="boost-text-over leaving"
                  onanimationend={finishStatusTransition}
                >
                  {statusTransitionTo}
                </span>
              {/if}
            {/key}
          {/if}
        </button>
        {#if boostSpinnerEnabled
          && startupIdentityPhase === "done"
          && spinnerAnimationVisible
          && displayedStatusText === "실시간 부스트중"
          && statusTransitionPhase === "done"}
          <span class="boost-progress" aria-hidden="true">
            <span></span>
            <span></span>
            <span></span>
            <span></span>
          </span>
        {/if}
        {#if startupIdentityPhase === "done"}
        <span class="status-guide">
          {#key guideTransitionId}
            {#if guideTransitionPhase === "out"}
              <span
                class="guide-text guide-text-out"
                onanimationend={() => guideTransitionPhase = "in"}
              >
                {guideTransitionFrom}
              </span>
            {:else if guideTransitionPhase === "in"}
              <span
                class="guide-text guide-text-in"
                onanimationend={() => guideTransitionPhase = "done"}
              >
                {guideTransitionTo}
              </span>
            {:else}
              <span class="guide-text">{displayedGuideText}</span>
            {/if}
          {/key}
        </span>
        {/if}
      </section>
      {#if colorTransition}
        {#key colorTransition.id}
          <section
            class="boost-home boost-color-overlay"
            class:paused={colorTransition.paused}
            style={`--wave-x: ${colorTransition.clientX}px; --wave-y: ${colorTransition.clientY}px; --wave-radius: ${colorTransition.radius}px; --wave-duration: ${colorTransition.duration}ms; --wave-delay: ${colorTransition.delay}ms`}
            aria-hidden="true"
            onanimationend={event => finishColorTransition(event, colorTransition.id)}
          >
            <img src="./logo3.png" alt="" draggable="false" />
          </section>
        {/key}
      {/if}
    {/if}
  {/if}
</main>

<main class="legacy-dashboard" aria-hidden="true">
  <header>
    <div>
      <p class="eyebrow">NOGIREM</p>
      <h1>게임 환경 최적화</h1>
      <p class="subtitle">현재 설정을 검사하고 필요한 항목만 적용합니다</p>
    </div>
    <button class="secondary" disabled={refreshing} onclick={loadAll}>
      {refreshing ? "확인 중…" : "전체 다시 확인"}
    </button>
  </header>

  <section class="cards" aria-label="최적화 상태">
    <article class="card frame-boost-card">
      <div class="card-heading">
        <div>
          <p class="category">게임 성능</p>
          <h2>프레임 부스트</h2>
        </div>
        {#if services.affinity.loading || services.memory.loading}
          <span class="status checking">확인 중</span>
        {:else if services.affinity.error || services.memory.error}
          <span class="status error">확인 실패</span>
        {:else if services.affinity.data?.running && services.memory.data?.running}
          <span class="status ready">실시간 부스트</span>
        {:else if services.affinity.data?.running || services.memory.data?.running}
          <span class="status needed">일부 실행</span>
        {:else}
          <span class="status idle">일시정지</span>
        {/if}
      </div>

      {#if services.affinity.error}
        <p class="error-message">{services.affinity.error}</p>
      {/if}
      {#if services.memory.error && services.memory.error !== services.affinity.error}
        <p class="error-message">{services.memory.error}</p>
      {/if}

      {#if services.affinity.data && services.memory.data}
        <p class="device">
          마비노기 CPU {services.affinity.data.gameCpuRange}
          · 백그라운드 CPU {services.affinity.data.backgroundCpuRange}
        </p>
        <div class="boost-sections">
          <section class="boost-section" aria-labelledby="affinity-boost-title">
            <h3 id="affinity-boost-title">Affinity 코어 분리</h3>
            <dl class="checks">
              <div>
                <dt>코어 감시</dt>
                <dd class:passed={services.affinity.data.running}>
                  {services.affinity.data.running ? "실행 중" : "일시정지"}
                </dd>
              </div>
              <div>
                <dt>마비노기 감지</dt>
                <dd class:passed={services.affinity.data.gameActive}>
                  {services.affinity.data.gameActive ? "실행 중" : "대기 중"}
                </dd>
              </div>
              <div>
                <dt>NIC RSS CPU 범위</dt>
                <dd class:passed={services.affinity.data.nic?.optimized}>
                  {!services.affinity.data.nic?.supported
                    ? "확인 불가"
                    : (services.affinity.data.nic.current?.enabled
                      ? `${services.affinity.data.nic.current.baseProcessorNumber ?? "자동"}-${services.affinity.data.nic.current.maxProcessorNumber ?? "자동"}`
                      : "RSS 꺼짐")}
                </dd>
              </div>
            </dl>

            <label class="option-row">
              <input
                type="checkbox"
                bind:checked={includeNic}
                disabled={services.affinity.data.running
                  || services.affinity.loading
                  || services.affinity.optimizing
                  || !services.affinity.data.nic?.supported}
              />
              <span>
                NIC RSS 최적화 포함
                <small>
                  {services.affinity.data.nic?.supported
                    ? `네트워크 수신 처리를 CPU ${services.affinity.data.backgroundCpuRange}에 배치`
                    : (services.affinity.data.nic?.reason ?? "NIC RSS 상태를 확인할 수 없음")}
                </small>
              </span>
            </label>
          </section>

          <section class="boost-section" aria-labelledby="memory-boost-title">
            <h3 id="memory-boost-title">메모리 최적화</h3>
            <dl class="checks network-checks">
              <div class="network-checks-heading">
                <dt>항목</dt>
                <dd>현재값</dd>
                <dd>정리 기준</dd>
              </div>
              <div>
                <dt>사용 가능 RAM</dt>
                <dd>{formatMiB(services.memory.data.current?.available)}</dd>
                <dd>&lt; {formatMiB(services.memory.data.thresholds?.availableTrigger)}</dd>
              </div>
              <div>
                <dt>Standby RAM</dt>
                <dd>{formatMiB(services.memory.data.current?.standby)}</dd>
                <dd>&gt; {formatMiB(services.memory.data.thresholds?.standbyTrigger)}</dd>
              </div>
              <div>
                <dt>메모리 감시</dt>
                <dd class:passed={services.memory.data.running}>
                  {services.memory.data.running ? "실행 중" : "일시정지"}
                </dd>
                <dd>{services.memory.data.gameActive ? "게임 감지" : "대기 중"}</dd>
              </div>
            </dl>
          </section>
        </div>
        <p class="notice">
          실시간 부스트는 Affinity와 메모리 감시를 함께 실행합니다. 일시정지는 두 감시를
          함께 멈추고 현재 CPU 배치를 유지합니다.
        </p>
        {#if services.affinity.data.error}
          <p class="error-message">{services.affinity.data.error.message}</p>
        {/if}
        {#if services.memory.data.error}
          <p class="error-message">
            {services.memory.data.error.message ?? services.memory.data.error}
          </p>
        {/if}
      {:else}
        <div class="skeleton"></div>
      {/if}

      <div class="actions frame-boost-actions">
        <button
          class:primary={!services.affinity.data?.running && !services.memory.data?.running}
          class:secondary={services.affinity.data?.running || services.memory.data?.running}
          disabled={services.affinity.loading
            || services.memory.loading
            || services.affinity.optimizing
            || services.memory.optimizing
            || !services.affinity.data
            || !services.memory.data}
          onclick={toggleFrameBoost}
        >
          {frameBoostAction === "run"
            ? "실행 중…"
            : (frameBoostAction === "pause"
              ? "일시정지 중…"
              : (services.affinity.data?.running || services.memory.data?.running
                ? "일시정지"
                : "실시간 부스트"))}
        </button>
        <button
          class="danger"
          disabled={services.affinity.loading
            || services.memory.loading
            || services.affinity.optimizing
            || services.memory.optimizing}
          onclick={resetFrameBoost}
        >
          {frameBoostAction === "reset" ? "정지 중…" : "정지(전체복구)"}
        </button>
      </div>
    </article>

    <article class="card">
      <div class="card-heading">
        <div>
          <p class="category">그래픽</p>
          <h2>NVIDIA 프로필</h2>
        </div>
        {#if services.nvidia.loading}
          <span class="status checking">확인 중</span>
        {:else if services.nvidia.error}
          <span class="status error">확인 실패</span>
        {:else if nvidiaReady(services.nvidia.data)}
          <span class="status ready">최적화됨</span>
        {:else}
          <span class="status needed">최적화 필요</span>
        {/if}
      </div>

      {#if services.nvidia.error}
        <p class="error-message">{services.nvidia.error}</p>
      {:else if services.nvidia.data}
        {#if services.nvidia.data.nvidia}
          <p class="device">
            {services.nvidia.data.gpus?.map(gpu => `${gpu.name} · ${gpu.driverVersion}`).join(", ")}
          </p>
          <dl class="checks">
            {#each Object.entries(goalNames) as [key, label]}
              <div>
                <dt>{label}</dt>
                <dd class:passed={services.nvidia.data.goals?.[key]}>
                  {services.nvidia.data.goals?.[key] ? "완료" : "필요"}
                </dd>
              </div>
            {/each}
          </dl>
          <p class="meta">
            프로필: {services.nvidia.data.profileName ?? "확인되지 않음"}
          </p>
        {:else}
          <p class="empty">{services.nvidia.data.reason ?? "NVIDIA GPU를 찾지 못했습니다"}</p>
        {/if}
      {:else}
        <div class="skeleton"></div>
      {/if}

      <div class="actions">
        <button
          class="secondary compact"
          disabled={services.nvidia.loading || services.nvidia.optimizing}
          onclick={() => refreshService("nvidia", window.nogirem.refreshNvidia)}
        >
          다시 확인
        </button>
        <button
          class="primary"
          disabled={services.nvidia.loading
            || services.nvidia.optimizing
            || !services.nvidia.data?.nvidia
            || nvidiaReady(services.nvidia.data)}
          onclick={() => optimize("nvidia", window.nogirem.optimizeNvidia)}
        >
          {services.nvidia.optimizing ? "적용 중…" : "NVIDIA 최적화"}
        </button>
      </div>
    </article>

    <article class="card">
      <div class="card-heading">
        <div>
          <p class="category">네트워크</p>
          <h2>네트워크 최적화</h2>
        </div>
        {#if services.network.loading}
          <span class="status checking">확인 중</span>
        {:else if services.network.error}
          <span class="status error">확인 실패</span>
        {:else if networkReady(services.network.data)}
          <span class="status ready">최적화됨</span>
        {:else}
          <span class="status needed">최적화 필요</span>
        {/if}
      </div>

      {#if services.network.error}
        <p class="error-message">{services.network.error}</p>
      {:else if services.network.data}
        <p class="device">
          {services.network.data.fastPing.current?.interfaceAlias}
          #{services.network.data.fastPing.current?.interfaceIndex}
        </p>
        <dl class="checks network-checks">
          <div class="network-checks-heading">
            <dt>옵션</dt>
            <dd>현재값</dd>
            <dd>권장값</dd>
          </div>
          <div>
            <dt>TCP ACK 빈도</dt>
            <dd class:passed={services.network.data.fastPing.current?.TcpAckFrequency === 1}>
              {services.network.data.fastPing.current?.TcpAckFrequency ?? "없음"}
            </dd>
            <dd class="passed">1</dd>
          </div>
          <div>
            <dt>TCP No Delay</dt>
            <dd class:passed={services.network.data.fastPing.current?.TCPNoDelay === 1}>
              {services.network.data.fastPing.current?.TCPNoDelay ?? "없음"}
            </dd>
            <dd class="passed">1</dd>
          </div>
          <div>
            <dt>TCP 자동 조정</dt>
            <dd class:passed={services.network.data.tcpAutoTuning.optimized}>
              {services.network.data.tcpAutoTuning.current?.effective ?? "확인 불가"}
            </dd>
            <dd class="passed">Normal</dd>
          </div>
        </dl>
        <p class="notice">
          앱 시작 시 승인한 관리자 권한으로 필요한 설정만 적용합니다.
          패스트핑 변경 시 네트워크가 잠시 끊길 수 있습니다.
        </p>
      {:else}
        <div class="skeleton"></div>
      {/if}

      <div class="actions">
        <button
          class="secondary compact"
          disabled={services.network.loading || services.network.optimizing}
          onclick={() => refreshService("network", window.nogirem.refreshNetwork)}
        >
          다시 확인
        </button>
        <button
          class="primary"
          disabled={services.network.loading
            || services.network.optimizing
            || !services.network.data
            || networkReady(services.network.data)}
          onclick={requestNetworkOptimization}
        >
          {services.network.optimizing ? "적용 중…" : "네트워크 최적화"}
        </button>
      </div>
    </article>

  </section>

  <footer>Windows x64 · 설정 적용 후 상태를 다시 검증합니다</footer>
</main>

<GameWave
  bind:this={gameWave}
  onplaybackstart={handleStartupPlaybackStart}
  onstartupcomplete={handleStartupComplete}
  onstartuphidden={handleStartupHidden}
  onstartuplogomaskchange={active => startupLogoMaskActive = active}
/>

{#if testVersionNoticeVisible}
  <NoticeModal
    title="미공개 테스트 버전 안내"
    onconfirm={() => testVersionNoticeVisible = false}
  >
    <p>미공개 테스트 버전은 9월 11일까지만 사용할 수 있습니다.</p>
  </NoticeModal>
{/if}

{#if conflictModalVisible}
  <Modal
    title="충돌 우려 프로그램 감지"
    closeSignal={conflictModalCloseSignal}
    onclose={dismissConflictWarning}
  >
    <p class="modal-description conflict-description">
      현재 필요하지 않거나 기능상 충돌이 발생할 수 있는 프로그램이 실행중입니다.
    </p>
    <ul class="conflict-program-list">
      {#each conflictPrograms as program}
        <li>{program}</li>
      {/each}
    </ul>
    <div class="conflict-modal-actions">
      <button
        class="secondary"
        onclick={() => conflictModalCloseSignal += 1}
      >
        무시하고 계속하기
      </button>
    </div>
  </Modal>
{/if}

{#if optimizationModalVisible}
  <Modal
    eyebrow="시스템 최적화"
    title="최적화 상태"
    variant="large"
    hideTitle={true}
    onclose={() => optimizationModalVisible = false}
  >
    <div class="optimization-modal-grid">
      <section class="optimization-detail" aria-labelledby="graphics-status-title">
        <header>
          <div class="detail-heading-copy">
            <span class="detail-category">그래픽 설정</span>
            <h3 id="graphics-status-title">NVIDIA 프로필</h3>
          </div>
          <button
            class="detail-action"
            class:complete={nvidiaReady(services.nvidia.data)}
            disabled={services.nvidia.loading
              || services.nvidia.optimizing
              || !services.nvidia.data?.nvidia
              || nvidiaReady(services.nvidia.data)}
            onclick={() => optimize("nvidia", window.nogirem.optimizeNvidia)}
          >
            {services.nvidia.loading
              ? "확인 중"
              : (services.nvidia.optimizing
                ? "적용 중"
                : (nvidiaReady(services.nvidia.data) ? "완료됨" : "최적화"))}
          </button>
        </header>
        {#if services.nvidia.error}
          <p class="detail-error">{services.nvidia.error}</p>
        {:else if services.nvidia.data?.nvidia}
          <p class="detail-device">
            {services.nvidia.data.gpus?.map(gpu => gpu.name).join(", ")}
          </p>
          <dl class="detail-list">
            {#each Object.entries(goalNames) as [key, label]}
              <div>
                <dt>{label}</dt>
                <dd class:ready={services.nvidia.data.goals?.[key]}>
                  {services.nvidia.data.goals?.[key] ? "완료" : "조정 필요"}
                </dd>
              </div>
            {/each}
          </dl>
        {:else}
          <p class="detail-empty">
            {services.nvidia.loading
              ? "그래픽 설정을 확인하고 있습니다"
              : (services.nvidia.data?.reason ?? "NVIDIA GPU를 찾지 못했습니다")}
          </p>
        {/if}
      </section>

      <section class="optimization-detail" aria-labelledby="network-status-title">
        <header>
          <div class="detail-heading-copy">
            <span class="detail-category">네트워크 상태</span>
            <h3 id="network-status-title">패스트핑 적용</h3>
          </div>
          <button
            class="detail-action"
            class:complete={networkReady(services.network.data)}
            disabled={services.network.loading
              || services.network.optimizing
              || !services.network.data
              || networkReady(services.network.data)}
            onclick={requestNetworkOptimization}
          >
            {services.network.loading
              ? "확인 중"
              : (services.network.optimizing
                ? "적용 중"
                : (networkReady(services.network.data) ? "완료됨" : "최적화"))}
          </button>
        </header>
        {#if services.network.error}
          <p class="detail-error">{services.network.error}</p>
        {:else if services.network.data}
          <p class="detail-device">
            {services.network.data.fastPing.current?.interfaceAlias ?? "기본 네트워크"}
          </p>
          <dl class="detail-list">
            <div>
              <dt>TCP ACK 빈도</dt>
              <dd class:ready={services.network.data.fastPing.current?.TcpAckFrequency === 1}>
                {services.network.data.fastPing.current?.TcpAckFrequency === 1 ? "완료" : "조정 필요"}
              </dd>
            </div>
            <div>
              <dt>TCP No Delay</dt>
              <dd class:ready={services.network.data.fastPing.current?.TCPNoDelay === 1}>
                {services.network.data.fastPing.current?.TCPNoDelay === 1 ? "완료" : "조정 필요"}
              </dd>
            </div>
            <div>
              <dt>TCP 자동 조정</dt>
              <dd class:ready={services.network.data.tcpAutoTuning.optimized}>
                {services.network.data.tcpAutoTuning.optimized ? "완료" : "조정 필요"}
              </dd>
            </div>
          </dl>
        {:else}
          <p class="detail-empty">네트워크 상태를 확인하고 있습니다</p>
        {/if}
      </section>
    </div>
  </Modal>
{/if}

{#if networkReconnectModalVisible}
  <Modal
    eyebrow="네트워크 최적화"
    title="네트워크 연결을 다시 시작합니다"
    hideClose={true}
    closeSignal={networkReconnectCloseSignal}
    onclose={finishNetworkReconnectModal}
  >
    <p class="modal-description">
      TCP ACK 빈도 또는 TCP No Delay를 적용하려면 네트워크 어댑터를 다시 연결해야 합니다.
      인터넷 연결이 잠시 끊길 수 있습니다. 계속하시겠습니까?
    </p>
    <div class="modal-actions">
      <button
        class="secondary"
        disabled={Boolean(networkReconnectAction)}
        onclick={() => closeNetworkReconnectModal("cancel")}
      >
        취소
      </button>
      <button
        class="monochrome"
        disabled={Boolean(networkReconnectAction)}
        onclick={() => closeNetworkReconnectModal("continue")}
      >
        계속하기
      </button>
    </div>
  </Modal>
{/if}

{#if closeModalVisible}
  <Modal
    eyebrow="프로그램 종료"
    title="프레임 부스트를 정지할까요?"
    closeDisabled={closeActionPending}
    onclose={() => confirmClose("cancel")}
  >
    <p class="modal-description">
      부스트를 유지한 채로 종료하면 마비노기 외 프로그램들의 성능이 제한될 수 있습니다.
    </p>
    <div class="modal-actions">
      <button
        class="danger"
        disabled={closeActionPending}
        onclick={() => confirmClose("reset")}
      >
        {closeActionPending ? "종료 준비 중…" : "부스트 설정 복구 후 종료"}
      </button>
      <button
        class="secondary"
        disabled={closeActionPending}
        onclick={() => confirmClose("keep")}
      >
        적용 유지 후 종료
      </button>
    </div>
  </Modal>
{/if}
