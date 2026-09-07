<script>
  import { onMount } from "svelte"
  import { fade, fly } from "svelte/transition"
  import packageInfo from "../package.json"
  import introduceMarkdown from "../INTRODUCE.md?raw"
  import operationMarkdown from "../OPERATION.md?raw"
  import turboKeyTermsMarkdown from "../TURBO_KEY_TERMS.md?raw"
  import versionHistoryMarkdown from "../VERSION_HISTORY.md?raw"
  import {
    defaultTurboKeyCodes,
    defaultTurboKeyIntervalMs,
    turboKeyIntervalOptions,
  } from "../src/turbo-key-settings.mjs"
  import creatorChannelAvatarUrl from "./creator-channel-avatar.jpg"
  import directDonationLogoUrl from "./direct-donation-logo.svg"
  import GameWave from "./GameWave.svelte"
  import MarkdownBlocks from "./MarkdownBlocks.svelte"
  import Modal from "./Modal.svelte"
  import TermsModal from "./TermsModal.svelte"
  import UpdatePreviewModal from "./UpdatePreviewModal.svelte"

  const keyboardRows = [
    [
      { label: "Esc", disabled: true }, { spacer: 0.5 },
      ...Array.from({ length: 12 }, (_, index) => ({
        label: `F${index + 1}`,
        code: 112 + index,
        groupStart: [0, 4, 8].includes(index),
      })),
      { spacer: 0.5 },
      { label: "Prt", disabled: true }, { label: "Scr", disabled: true }, { label: "Pause", disabled: true },
    ],
    [
      { label: "`", code: 192 },
      ...["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"].map(label => ({
        label,
        code: label === "0" ? 48 : 48 + Number(label),
      })),
      { label: "-", code: 189 }, { label: "=", code: 187 }, { label: "Back", code: 8, units: 2 },
      { spacer: 0.5 },
      { label: "Ins", code: 45 }, { label: "Home", code: 36 }, { label: "PgUp", code: 33 },
      { spacer: 0.5 },
      { label: "Num", disabled: true }, { label: "/", code: 111 }, { label: "*", code: 106 }, { label: "-", code: 109 },
    ],
    [
      { label: "Tab", code: 9, units: 1.5 },
      ...Array.from("QWERTYUIOP").map(label => ({ label, code: label.charCodeAt(0) })),
      { label: "[", code: 219 }, { label: "]", code: 221 }, { label: "\\", code: 220, units: 1.5 },
      { spacer: 0.5 },
      { label: "Del", code: 46 }, { label: "End", code: 35 }, { label: "PgDn", code: 34 },
      { spacer: 0.5 },
      { label: "7", code: 103 }, { label: "8", code: 104 }, { label: "9", code: 105 }, { label: "+", code: 107 },
    ],
    [
      { label: "Caps", disabled: true, units: 1.8 },
      ...Array.from("ASDFGHJKL").map(label => ({ label, code: label.charCodeAt(0) })),
      { label: ";", code: 186 }, { label: "'", code: 222 }, { label: "Enter", code: 13, units: 2.2 },
      { spacer: 4 },
      { label: "4", code: 100 }, { label: "5", code: 101 }, { label: "6", code: 102 }, { label: "+", code: 107 },
    ],
    [
      { label: "Shift", disabled: true, units: 2.3 },
      ...Array.from("ZXCVBNM").map(label => ({ label, code: label.charCodeAt(0) })),
      { label: ",", code: 188 }, { label: ".", code: 190 }, { label: "/", code: 191 },
      { label: "Shift", disabled: true, units: 2.7 },
      { spacer: 1.5 },
      { label: "↑", code: 38 },
      { spacer: 1.5 },
      { label: "1", code: 97 }, { label: "2", code: 98 }, { label: "3", code: 99 }, { label: "Enter", code: 13 },
    ],
    [
      { label: "Ctrl", disabled: true, units: 1.4 }, { label: "Win", disabled: true, units: 1.3 },
      { label: "Alt", disabled: true, units: 1.3 }, { label: "Space", code: 32, units: 6.2 },
      { label: "Alt", disabled: true, units: 1.3 }, { label: "Win", disabled: true, units: 1.3 },
      { label: "Menu", disabled: true, units: 1.3 }, { label: "Ctrl", disabled: true, units: 1.4 },
      { spacer: 0.5 },
      { label: "←", code: 37 }, { label: "↓", code: 40 }, { label: "→", code: 39 },
      { spacer: 0.5 },
      { label: "0", code: 96, units: 2 }, { label: ".", code: 110 }, { label: "Enter", code: 13 },
    ],
  ]
  const functionKeyboardRow = keyboardRows[0]
  const mainKeyboardRows = keyboardRows.slice(1).map(row => {
    const spacerIndex = row.findIndex(key => key.spacer)
    return spacerIndex < 0 ? row : row.slice(0, spacerIndex)
  })
  const navigationKeys = [
    { label: "Ins", code: 45, row: 1, column: 1 },
    { label: "Home", code: 36, row: 1, column: 2 },
    { label: "PgUp", code: 33, row: 1, column: 3 },
    { label: "Del", code: 46, row: 2, column: 1 },
    { label: "End", code: 35, row: 2, column: 2 },
    { label: "PgDn", code: 34, row: 2, column: 3 },
    { label: "↑", code: 38, row: 4, column: 2 },
    { label: "←", code: 37, row: 5, column: 1 },
    { label: "↓", code: 40, row: 5, column: 2 },
    { label: "→", code: 39, row: 5, column: 3 },
  ]
  const numpadKeys = [
    { label: "Num", disabled: true, row: 1, column: 1 },
    { label: "/", code: 111, row: 1, column: 2 },
    { label: "*", code: 106, row: 1, column: 3 },
    { label: "-", code: 109, row: 1, column: 4 },
    { label: "7", code: 103, row: 2, column: 1 },
    { label: "8", code: 104, row: 2, column: 2 },
    { label: "9", code: 105, row: 2, column: 3 },
    { label: "+", code: 107, row: 2, column: 4, rowSpan: 2 },
    { label: "4", code: 100, row: 3, column: 1 },
    { label: "5", code: 101, row: 3, column: 2 },
    { label: "6", code: 102, row: 3, column: 3 },
    { label: "1", code: 97, row: 4, column: 1 },
    { label: "2", code: 98, row: 4, column: 2 },
    { label: "3", code: 99, row: 4, column: 3 },
    { label: "Enter", code: 13, row: 4, column: 4, rowSpan: 2 },
    { label: "0", code: 96, row: 5, column: 1, columnSpan: 2 },
    { label: ".", code: 110, row: 5, column: 3 },
  ]
  const selectableKeyboardCodes = new Set(
    [...functionKeyboardRow, ...mainKeyboardRows.flat(), ...navigationKeys, ...numpadKeys]
      .filter(key => key.code && !key.disabled)
      .map(key => key.code),
  )

  let services = {
    graphics: { loading: true, data: null, error: null },
    network: { loading: true, data: null, error: null },
    affinity: { loading: true, data: null, error: null },
    memory: { loading: true, data: null, error: null },
  }
  let refreshing = false
  let includeNic = false
  let affinityRuntimeSyncing = false
  let memoryRuntimeSyncing = false
  let frameBoostAction = null
  let cpuReorderAction = null
  let cpuReorderNotice = ""
  let gameCpuCoreAction = null
  let gameCpuCoreNotice = ""
  let diagnosticLogAction = null
  let diagnosticLogNotice = ""
  let startupTrayEnabled = false
  let startupTraySupported = false
  let startupTraySettingLoaded = false
  let startupTrayAction = null
  let startupTrayNotice = ""
  let turboKeyEnabled = false
  let turboKeyRunning = false
  let turboKeyInstalled = false
  let turboKeySettingLoaded = false
  let turboKeyAction = null
  let turboKeyNotice = ""
  let turboKeyCodes = []
  let turboKeyDraftCodes = []
  let turboKeyIntervalMs = defaultTurboKeyIntervalMs
  let turboKeyDraftIntervalMs = defaultTurboKeyIntervalMs
  let turboKeyModalVisible = false
  let turboKeyModalCloseSignal = 0
  let turboKeyEnableAfterSettings = false
  let turboTermsModalVisible = false
  let turboTermsModalCloseSignal = 0
  let turboInstallAction = null
  let blackboxSettingLoaded = false
  let blackboxFeatureEnabled = false
  let blackboxFeatureAction = null
  let blackboxFeatureNotice = ""
  let blackboxEnabled = false
  let blackboxTogglePending = false
  let blackboxDisplayedText = "블랙박스 꺼짐"
  let blackboxDisplayedEnabled = false
  let blackboxTransitionFrom = "블랙박스 꺼짐"
  let blackboxTransitionTo = "블랙박스 꺼짐"
  let blackboxTransitionFromEnabled = false
  let blackboxTransitionToEnabled = false
  let blackboxTransitionPhase = "done"
  let blackboxTransitionId = 0
  let closeModalVisible = false
  let closeModalCloseSignal = 0
  let optimizationModalVisible = false
  let optimizationModalCloseSignal = 0
  let conflictModalVisible = false
  let networkReconnectModalVisible = false
  let networkReconnectCloseSignal = 0
  let networkReconnectAction = null
  let networkReconnectMode = "apply"
  let radeonGlobalModalVisible = false
  let radeonGlobalCloseSignal = 0
  let radeonGlobalAction = null
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
  let windowVisuallyActive = false
  let applicationUpdateState = {
    phase: "idle",
    percent: 0,
    version: null,
    error: null,
  }
  let applicationUpdateInstalling = false
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
  let creatorNavigationTimer
  let creatorNavigationReady = false
  let creatorPromptStateLoaded = false
  let creatorPromptDismissed = false
  let creatorPromptDisplayRecorded = false
  let creatorViewPhase = "home"
  let creatorTab = "developer"
  let pendingCreatorTab = null
  let creatorTabTransitionPhase = "idle"
  let creatorContentElement
  let creatorScrollTarget = 0
  let creatorScrollFrame
  let creatorChannelProfile = null

  const versionText = "공개 사용자 버전"
  const ambientRhythmEnabled = true
  const boostSpinnerEnabled = true
  const creatorSections = [
    { id: "developer", label: "소개", content: [] },
    { id: "operation", label: "작동 원리", content: [] },
    { id: "donation", label: "후원 / 기부", content: [] },
    { id: "history", label: "버전 변경 기록", content: [] },
    { id: "developer-tools", label: "고급 기능", content: [] },
  ]
  const operationPolicyUrl = "https://mabinogi.nexon.com/page/archive/guide_view.asp?id=4889849&num=7&playtarget=1"
  const introduceBlocks = parseIntroduceMarkdown(introduceMarkdown)
  const operationBlocks = parseIntroduceMarkdown(operationMarkdown)
  const turboTermsBlocks = parseIntroduceMarkdown(turboKeyTermsMarkdown)
  const versionHistoryEntries = parseVersionHistory(versionHistoryMarkdown)

  $: creatorPromptVisible = interfaceVisible
    && !settingsVisible
    && startupIdentityPhase === "done"
    && creatorNavigationReady
    && creatorPromptStateLoaded
    && !creatorPromptDismissed
    && creatorViewPhase === "home"
    && pageVisible

  $: if (creatorPromptVisible && !creatorPromptDisplayRecorded) {
    creatorPromptDisplayRecorded = true
    void window.nogirem.recordCreatorPromptDisplay().catch(() => {})
  }

  function decodeMarkdownText(text) {
    return text.replace(/\\([\\`*_[\]{}()#+\-.!])/g, "$1")
  }

  function parseIntroduceMarkdown(markdown) {
    const blocks = []
    let paragraph = []
    let listItems = []
    const source = markdown.replace(/<!--[\s\S]*?-->/g, "")

    const flushParagraph = () => {
      if (!paragraph.length) return
      blocks.push({ type: "paragraph", text: paragraph.join(" ") })
      paragraph = []
    }
    const flushList = () => {
      if (!listItems.length) return
      blocks.push({ type: "list", items: listItems })
      listItems = []
    }

    for (const sourceLine of source.split(/\r?\n/)) {
      const line = sourceLine.trim()
      const headingMatch = line.match(/^(#{1,3})\s+(.+)$/)
      const listMatch = line.match(/^[-*]\s+(.+)$/)
      const imageMatch = line.match(/^!\[([^\]]*)\]\((\.\/doc_operation\/[^)\s]+)\)$/)
      const linkMatch = line.match(/^\[([^\]]+)\]\((https:\/\/[^)\s]+)\)$/)

      if (linkMatch?.[2] === operationPolicyUrl) {
        flushParagraph()
        flushList()
        blocks.push({
          type: "link",
          text: decodeMarkdownText(linkMatch[1]),
          href: linkMatch[2],
        })
      } else if (imageMatch) {
        flushParagraph()
        flushList()
        blocks.push({
          type: "image",
          alt: decodeMarkdownText(imageMatch[1]),
          src: imageMatch[2],
        })
      } else if (headingMatch) {
        flushParagraph()
        flushList()
        blocks.push({
          type: "heading",
          level: headingMatch[1].length,
          text: decodeMarkdownText(headingMatch[2]),
        })
      } else if (listMatch) {
        flushParagraph()
        listItems.push(decodeMarkdownText(listMatch[1]))
      } else if (!line) {
        flushParagraph()
        flushList()
      } else {
        flushList()
        paragraph.push(decodeMarkdownText(line))
      }
    }
    flushParagraph()
    flushList()
    return blocks
  }

  function parseVersionHistory(markdown) {
    const entries = []
    let currentEntry = null

    for (const sourceLine of markdown.split(/\r?\n/)) {
      const line = sourceLine.trim()
      const versionMatch = line.match(/^##\s+(.+)$/)

      if (versionMatch) {
        currentEntry = { version: versionMatch[1], changes: [] }
        entries.push(currentEntry)
      } else if (currentEntry && /^[-*]\s+/.test(line)) {
        currentEntry.changes.push(line.replace(/^[-*]\s+/, ""))
      }
    }

    return entries
  }

  function activeCreatorSection() {
    return creatorSections.find(section => section.id === creatorTab) ?? creatorSections[0]
  }

  function selectCreatorTab(nextTab) {
    if (
      nextTab === creatorTab
      || creatorTabTransitionPhase !== "idle"
    ) return
    pendingCreatorTab = nextTab
    creatorTabTransitionPhase = "leaving"
  }

  function stopCreatorScroll() {
    if (creatorScrollFrame) cancelAnimationFrame(creatorScrollFrame)
    creatorScrollFrame = null
  }

  function animateCreatorScroll() {
    if (!creatorContentElement) {
      creatorScrollFrame = null
      return
    }
    const distance = creatorScrollTarget - creatorContentElement.scrollTop
    if (Math.abs(distance) < 0.5) {
      creatorContentElement.scrollTop = creatorScrollTarget
      creatorScrollFrame = null
      return
    }
    creatorContentElement.scrollTop += distance * 0.18
    creatorScrollFrame = requestAnimationFrame(animateCreatorScroll)
  }

  function smoothCreatorScroll(event) {
    if (event.ctrlKey || !creatorContentElement) return
    const maximum = creatorContentElement.scrollHeight - creatorContentElement.clientHeight
    if (maximum <= 0) return
    const unit = event.deltaMode === 1
      ? 16
      : event.deltaMode === 2
        ? creatorContentElement.clientHeight
        : 1
    const start = creatorScrollFrame
      ? creatorScrollTarget
      : creatorContentElement.scrollTop
    const next = Math.max(0, Math.min(maximum, start + event.deltaY * unit * 0.8))
    if (next === start) return
    event.preventDefault()
    creatorScrollTarget = next
    if (!creatorScrollFrame) {
      creatorScrollFrame = requestAnimationFrame(animateCreatorScroll)
    }
  }

  function finishCreatorTabTransition(event) {
    if (event.target !== event.currentTarget) return
    if (
      creatorTabTransitionPhase === "leaving"
      && event.animationName === "creator-tab-out-left"
    ) {
      creatorTab = pendingCreatorTab
      pendingCreatorTab = null
      stopCreatorScroll()
      creatorScrollTarget = 0
      if (creatorContentElement) creatorContentElement.scrollTop = 0
      creatorTabTransitionPhase = "entering"
    } else if (
      creatorTabTransitionPhase === "entering"
      && event.animationName === "creator-tab-in-right"
    ) {
      creatorTabTransitionPhase = "idle"
    }
  }

  function creatorChannelStatistics() {
    if (
      !Number.isFinite(creatorChannelProfile?.subscriberCount)
      || !Number.isFinite(creatorChannelProfile?.videoCount)
    ) {
      return "YouTube 채널"
    }
    return `구독자 ${creatorChannelProfile.subscriberCount.toLocaleString("ko-KR")}명 · 동영상 ${creatorChannelProfile.videoCount.toLocaleString("ko-KR")}개`
  }

  function openCreatorView() {
    if (
      creatorViewPhase !== "home"
      || !interfaceVisible
      || settingsVisible
      || startupIdentityPhase !== "done"
      || !creatorNavigationReady
      || colorTransition
    ) return
    creatorPromptDismissed = true
    void window.nogirem.dismissCreatorPrompt().catch(() => {})
    creatorViewPhase = "opening"
  }

  function closeCreatorView() {
    if (creatorViewPhase !== "open") return
    creatorViewPhase = "closing"
  }

  function toggleCreatorView() {
    if (creatorViewPhase === "home") openCreatorView()
    else if (creatorViewPhase === "open") closeCreatorView()
  }

  function finishCreatorViewTransition(event) {
    if (event.target !== event.currentTarget || event.animationName === "") return
    if (creatorViewPhase === "opening") creatorViewPhase = "open"
    else if (creatorViewPhase === "closing") creatorViewPhase = "home"
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
    creatorNavigationReady = false
    interfaceVisible = true
    syncConflictWarning(
      services.affinity.data?.conflictingPrograms,
      statusText === "실시간 부스트중",
    )
    window.clearTimeout(startupIdentityTimer)
    window.clearTimeout(creatorNavigationTimer)
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
      window.clearTimeout(creatorNavigationTimer)
      creatorNavigationTimer = window.setTimeout(() => {
        creatorNavigationReady = true
      }, 3000)
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
    if (services.affinity.loading || services.memory.loading) {
      return "부스트 대기중"
    }
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

  function cpuReorderAvailable() {
    return services.affinity.data?.running
      && services.memory.data?.running
      && services.affinity.data?.gameActive
      && !frameBoostAction
      && !cpuReorderAction
      && !gameCpuCoreAction
  }

  function gameCpuCoreOptions() {
    const max = services.affinity.data?.gameCoreSetting?.maxGameCoreCount
    return Number.isInteger(max)
      ? Array.from({ length: max }, (_, index) => index + 1)
      : []
  }

  function gameCpuCoreColor(coreCount) {
    const max = services.affinity.data?.gameCoreSetting?.maxGameCoreCount
    const progress = Number.isInteger(max) && max > 1
      ? (coreCount - 1) / (max - 1)
      : 0
    const hue = Math.round(120 * (1 - progress))
    return `hsl(${hue} 58% 42%)`
  }

  function gameCpuUnavailableCores() {
    const setting = services.affinity.data?.gameCoreSetting
    const performanceCoreCount = setting?.performanceCoreCount
    const efficiencyCoreCount = setting?.efficiencyCoreCount
    if (!Number.isInteger(performanceCoreCount) || !Number.isInteger(efficiencyCoreCount)) {
      return []
    }
    const unavailable = []
    if (performanceCoreCount > 1) {
      unavailable.push({
        label: `P${performanceCoreCount}`,
        reason: "백그라운드와 입력 프로그램을 위해 남겨 두는 P-core",
      })
    }
    for (let index = 1; index <= efficiencyCoreCount; index++) {
      unavailable.push({
        label: `E${performanceCoreCount + index}`,
        reason: "마비노기에 배정하지 않는 E-core",
      })
    }
    return unavailable
  }

  async function selectGameCpuCoreCount(gameCoreCount) {
    if (gameCpuCoreAction || services.affinity.data?.cpuReorder?.state === "running") return
    gameCpuCoreAction = "saving"
    gameCpuCoreNotice = ""
    try {
      const affinity = await window.nogirem.setGameCpuCoreCount(gameCoreCount)
      updateService("affinity", {
        loading: false,
        data: affinity,
        error: null,
      })
      gameCpuCoreNotice = `마비노기에 물리 코어 ${gameCoreCount}개를 우선 배정합니다`
    } catch (error) {
      gameCpuCoreNotice = messageOf(error)
    } finally {
      gameCpuCoreAction = null
    }
  }

  async function runCpuReorder() {
    if (!cpuReorderAvailable()) return
    cpuReorderAction = "running"
    cpuReorderNotice = ""
    updateService("affinity", { error: null })
    try {
      const runtime = await window.nogirem.runCpuReorder()
      updateService("affinity", {
        data: {
          ...services.affinity.data,
          ...runtime,
        },
        error: null,
      })
      cpuReorderNotice = "CPU 재정렬이 완료되었습니다"
    } catch (error) {
      const message = messageOf(error)
      cpuReorderNotice = message
      updateService("affinity", { error: message })
    } finally {
      cpuReorderAction = null
    }
  }

  async function exportDiagnosticLogs() {
    if (diagnosticLogAction) return
    diagnosticLogAction = "exporting"
    diagnosticLogNotice = ""
    try {
      const result = await window.nogirem.exportDiagnosticLogs()
      if (!result.canceled) {
        const fileName = result.filePath.split(/[\\/]/).pop()
        diagnosticLogNotice = `${fileName} 저장 완료`
      }
    } catch (error) {
      diagnosticLogNotice = messageOf(error)
    } finally {
      diagnosticLogAction = null
    }
  }

  async function toggleStartupTray() {
    if (!startupTraySupported || startupTrayAction) return
    startupTrayAction = "saving"
    startupTrayNotice = ""
    try {
      const state = await window.nogirem.setStartupTraySetting(!startupTrayEnabled)
      startupTrayEnabled = Boolean(state.enabled)
      startupTraySupported = Boolean(state.supported)
    } catch (error) {
      startupTrayNotice = messageOf(error)
    } finally {
      startupTrayAction = null
    }
  }

  function applyTurboKeyState(state) {
    turboKeyInstalled = Boolean(state.installed)
    turboKeyEnabled = Boolean(state.enabled)
    turboKeyRunning = Boolean(state.running)
    turboKeyCodes = state.keys
    turboKeyIntervalMs = state.intervalMs
    if (state.reason) turboKeyNotice = state.reason
  }

  function openTurboTerms() {
    if (!turboKeySettingLoaded || turboKeyInstalled || turboInstallAction) return
    turboKeyNotice = ""
    turboTermsModalVisible = true
  }

  async function downloadTurboKey() {
    if (turboInstallAction || turboKeyInstalled) return
    turboInstallAction = "downloading"
    turboKeyNotice = ""
    let installed = false
    try {
      const state = await window.nogirem.downloadTurboKeyHelper()
      applyTurboKeyState(state)
      installed = state.installed
    } catch (error) {
      turboKeyNotice = messageOf(error)
    } finally {
      turboInstallAction = null
      if (installed) turboTermsModalCloseSignal++
    }
  }

  async function removeTurboKey() {
    if (turboKeyAction || !turboKeyInstalled) return
    turboKeyAction = "removing"
    turboKeyNotice = ""
    try {
      const state = await window.nogirem.removeTurboKeyHelper()
      applyTurboKeyState(state)
      turboKeyModalVisible = false
    } catch (error) {
      turboKeyNotice = messageOf(error)
    } finally {
      turboKeyAction = null
    }
  }

  function openOperationPolicyLink() {
    void window.nogirem.openOperationPolicy()
  }

  async function toggleTurboKey() {
    if (!turboKeySettingLoaded || !turboKeyInstalled || turboKeyAction) return
    if (!turboKeyEnabled) {
      turboKeyEnableAfterSettings = true
      turboKeyDraftCodes = [...turboKeyCodes]
      turboKeyDraftIntervalMs = turboKeyIntervalMs
      turboKeyModalVisible = true
      return
    }
    turboKeyAction = "saving"
    turboKeyNotice = ""
    try {
      const state = await window.nogirem.setTurboKeySetting({
        enabled: false,
        keys: turboKeyCodes,
        intervalMs: turboKeyIntervalMs,
      })
      applyTurboKeyState(state)
    } catch (error) {
      turboKeyNotice = messageOf(error)
    } finally {
      turboKeyAction = null
    }
  }

  function openTurboKeySettings() {
    if (!turboKeySettingLoaded || !turboKeyInstalled || turboKeyAction) return
    turboKeyEnableAfterSettings = false
    turboKeyDraftCodes = [...turboKeyCodes]
    turboKeyDraftIntervalMs = turboKeyIntervalMs
    turboKeyModalVisible = true
  }

  function closeTurboKeySettings() {
    turboKeyModalVisible = false
    turboKeyEnableAfterSettings = false
  }

  function toggleTurboKeyCode(code) {
    if (turboKeyAction) return
    turboKeyDraftCodes = turboKeyDraftCodes.includes(code)
      ? turboKeyDraftCodes.filter(value => value !== code)
      : [...turboKeyDraftCodes, code]
  }

  function resetTurboKeyCodes() {
    if (turboKeyAction) return
    turboKeyDraftCodes = [...defaultTurboKeyCodes]
    turboKeyDraftIntervalMs = defaultTurboKeyIntervalMs
  }

  function handleTurboKeyPickerInput(event) {
    if (!turboKeyModalVisible || turboKeyAction) return
    const code = Number(event.keyCode)
    if (!selectableKeyboardCodes.has(code)) return
    event.preventDefault()
    event.stopPropagation()
    if (event.repeat) return
    toggleTurboKeyCode(code)
  }

  function closeTopLayerWithEscape() {
    if (closeModalVisible) {
      if (!closeActionPending) closeModalCloseSignal++
    } else if (networkReconnectModalVisible) {
      if (!networkReconnectAction) closeNetworkReconnectModal("cancel")
    } else if (radeonGlobalModalVisible) {
      if (!radeonGlobalAction) closeRadeonGlobalModal("cancel")
    } else if (turboTermsModalVisible) {
      if (!turboInstallAction) turboTermsModalCloseSignal++
    } else if (turboKeyModalVisible) {
      if (turboKeyAction !== "keys") turboKeyModalCloseSignal++
    } else if (conflictModalVisible) {
      conflictModalCloseSignal++
    } else if (optimizationModalVisible) {
      optimizationModalCloseSignal++
    } else if (creatorViewPhase === "open") {
      closeCreatorView()
    } else if (settingsVisible) {
      settingsVisible = false
    } else {
      return false
    }
    return true
  }

  function handleApplicationKeydown(event) {
    if (event.key === "Escape" && closeTopLayerWithEscape()) {
      event.preventDefault()
      event.stopImmediatePropagation()
      return
    }
    handleTurboKeyPickerInput(event)
  }

  async function saveTurboKeyCodes() {
    if (turboKeyAction) return
    turboKeyAction = "keys"
    turboKeyNotice = ""
    let saved = false
    try {
      const state = await window.nogirem.setTurboKeySetting({
        enabled: turboKeyEnableAfterSettings || turboKeyEnabled,
        keys: turboKeyDraftCodes,
        intervalMs: turboKeyDraftIntervalMs,
      })
      applyTurboKeyState(state)
      saved = true
      turboKeyEnableAfterSettings = false
    } catch (error) {
      turboKeyNotice = messageOf(error)
    } finally {
      turboKeyAction = null
      if (saved) turboKeyModalCloseSignal++
    }
  }

  async function syncTurboKeySetting() {
    if (!turboKeySettingLoaded || turboKeyAction || turboInstallAction) return
    try {
      const state = await window.nogirem.getTurboKeySetting()
      applyTurboKeyState(state)
    } catch {
    }
  }

  function applyBlackboxState(state) {
    if ("featureEnabled" in state) {
      blackboxFeatureEnabled = Boolean(state.featureEnabled)
    }
    const nextEnabled = Boolean(state.enabled)
    const nextText = nextEnabled ? "블랙박스 켜짐" : "블랙박스 꺼짐"
    const visualTarget = blackboxTransitionPhase === "done"
      ? blackboxDisplayedText
      : blackboxTransitionTo
    if (blackboxSettingLoaded && nextText !== visualTarget) {
      blackboxTransitionFrom = blackboxDisplayedText
      blackboxTransitionTo = nextText
      blackboxTransitionFromEnabled = blackboxDisplayedEnabled
      blackboxTransitionToEnabled = nextEnabled
      blackboxTransitionPhase = "enter"
      blackboxTransitionId += 1
    } else if (!blackboxSettingLoaded) {
      blackboxDisplayedText = nextText
      blackboxDisplayedEnabled = nextEnabled
      blackboxTransitionFrom = nextText
      blackboxTransitionTo = nextText
      blackboxTransitionFromEnabled = nextEnabled
      blackboxTransitionToEnabled = nextEnabled
      blackboxTransitionPhase = "done"
    }
    blackboxEnabled = nextEnabled
  }

  function revealBlackboxTransitionTarget() {
    blackboxDisplayedText = blackboxTransitionTo
    blackboxDisplayedEnabled = blackboxTransitionToEnabled
    blackboxTransitionPhase = "leave"
  }

  function holdBlackboxTransitionMask() {
    blackboxTransitionPhase = "hold"
    if (!blackboxTogglePending) revealBlackboxTransitionTarget()
  }

  function finishBlackboxTransition() {
    blackboxTransitionPhase = "done"
  }

  async function toggleMainBlackbox() {
    if (!blackboxSettingLoaded || blackboxTogglePending) return
    const nextEnabled = !blackboxEnabled
    blackboxTogglePending = true
    applyBlackboxState({ enabled: nextEnabled })
    try {
      applyBlackboxState(await window.nogirem.setBlackboxEnabled(nextEnabled))
    } catch (error) {
      console.error("메인 블랙박스 상태 전환 실패", error)
      try {
        applyBlackboxState(await window.nogirem.getBlackboxSetting())
      } catch {
      }
    } finally {
      blackboxTogglePending = false
      if (blackboxTransitionPhase === "hold") revealBlackboxTransitionTarget()
    }
  }

  async function toggleBlackboxFeature() {
    if (!blackboxSettingLoaded || blackboxFeatureAction) return
    blackboxFeatureAction = "saving"
    blackboxFeatureNotice = ""
    try {
      applyBlackboxState(
        await window.nogirem.setBlackboxFeatureEnabled(!blackboxFeatureEnabled),
      )
    } catch (error) {
      blackboxFeatureNotice = messageOf(error)
    } finally {
      blackboxFeatureAction = null
    }
  }

  async function syncBlackboxSetting() {
    if (!blackboxSettingLoaded || blackboxTogglePending) return
    try {
      applyBlackboxState(await window.nogirem.getBlackboxSetting())
    } catch {
    }
  }

  function guideTextForStatus(statusText) {
    if (statusText === "실시간 부스트중") {
      return "마비노기를 위해 모든 프로세스를 최적화 하고 있습니다"
    }
    if (statusText === "부스트 대기중") return "마비노기 클라이언트를 기다리고 있습니다"
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
    const nextPaused = isPausedStatus(nextStatusText)
    if (!colorTransition && nextPaused !== visualPaused) {
      visualPaused = nextPaused
      gameWave?.setPaused(nextPaused)
    }
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
  }

  async function loadAll() {
    refreshing = true
    services = {
      graphics: { ...services.graphics, loading: true, error: null },
      network: { ...services.network, loading: true, error: null },
      affinity: { ...services.affinity, loading: true, error: null },
      memory: { ...services.memory, loading: true, error: null },
    }
    try {
      const result = await window.nogirem.getStatus()
      receiveResult("graphics", result.graphics ?? result.nvidia)
      receiveResult("network", result.network)
      receiveResult("affinity", result.affinity)
      receiveResult("memory", result.memory)
      if (result.affinity.ok && result.affinity.data.running) {
        includeNic = result.affinity.data.includeNic
      }
    } catch (error) {
      const message = messageOf(error)
      updateService("graphics", { loading: false, error: message })
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
    networkReconnectMode = "apply"
    networkReconnectAction = null
    networkReconnectModalVisible = true
  }

  function requestNetworkRestore() {
    networkReconnectMode = "restore"
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
      void optimize(
        "network",
        networkReconnectMode === "restore"
          ? window.nogirem.restoreNetwork
          : window.nogirem.optimizeNetwork,
      )
    }
  }

  function graphicsReady(data) {
    return data?.supported && data?.detected && data?.allMet
  }

  function requestGraphicsOptimization() {
    if (services.graphics.data?.vendor !== "amd") {
      void optimize("graphics", window.nogirem.optimizeGraphics)
      return
    }
    optimizationModalVisible = false
    radeonGlobalAction = null
    radeonGlobalModalVisible = true
  }

  function closeRadeonGlobalModal(action) {
    if (radeonGlobalAction) return
    radeonGlobalAction = action
    radeonGlobalCloseSignal += 1
  }

  function finishRadeonGlobalModal() {
    const action = radeonGlobalAction
    radeonGlobalModalVisible = false
    radeonGlobalAction = null
    if (action === "continue") {
      void optimize("graphics", window.nogirem.optimizeGraphics)
    }
  }

  function networkReady(data) {
    return data?.optimized
  }

  function networkRestoreAvailable(data) {
    return data?.originalStateRecorded
      || (
        data?.fastPing?.supported !== false
        && (
          data?.fastPing?.current?.TcpAckFrequency === 1
          || data?.fastPing?.current?.TCPNoDelay === 1
        )
      )
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
    const enabled = !(services.affinity.data?.running && services.memory.data?.running)
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
    const enabled = !(services.affinity.data?.running && services.memory.data?.running)
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

  function minimizeApplicationToTray() {
    void window.nogirem.minimizeToTray().catch(error => {
      updateService("affinity", { error: messageOf(error) })
    })
  }

  async function syncAffinityRuntime() {
    if (
      closeActionPending
      || affinityRuntimeSyncing
      || gameCpuCoreAction
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
          backgroundCpuRange: runtime.backgroundCpuRange,
          gameCpuRange: runtime.gameCpuRange,
          cpuTopology: runtime.cpuTopology,
          gameCoreSetting: runtime.gameCoreSetting,
          gameCoreReconfigure: runtime.gameCoreReconfigure,
          conflictingPrograms: runtime.conflictingPrograms,
          cpuReorder: runtime.cpuReorder,
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

  async function installApplicationUpdate() {
    if (applicationUpdateInstalling) return
    applicationUpdateInstalling = true
    try {
      const started = await window.nogirem.installUpdate()
      if (!started) applicationUpdateInstalling = false
    } catch {
      applicationUpdateInstalling = false
    }
  }

  function checkApplicationUpdate() {
    void window.nogirem.checkUpdate().catch(() => {})
  }

  onMount(() => {
    document.addEventListener("visibilitychange", syncPageVisibility)
    window.addEventListener("keydown", handleApplicationKeydown, true)
    const removeGraphicsStatusListener = window.nogirem.onGraphicsStatusChanged(status => {
      updateService("graphics", { loading: false, data: status, error: null })
    })
    const removeVisualActivityListener = window.nogirem.onVisualActivityChanged(
      setWindowVisualActivity,
    )
    const removeUpdateStateListener = window.nogirem.onUpdateStateChanged(state => {
      applicationUpdateState = state
    })
    syncPageVisibility()
    void window.nogirem.getVisualActivity().then(setWindowVisualActivity)
    void window.nogirem.getUpdateState()
      .then(state => {
        applicationUpdateState = state
      })
      .catch(() => {})
    void window.nogirem.getCreatorPromptDismissed()
      .then(dismissed => {
        creatorPromptDismissed = Boolean(dismissed)
      })
      .catch(() => {})
      .finally(() => {
        creatorPromptStateLoaded = true
      })
    void window.nogirem.getCreatorChannel()
      .then(profile => {
        creatorChannelProfile = profile
      })
      .catch(() => {})
    void window.nogirem.getStartupTraySetting()
      .then(state => {
        startupTrayEnabled = Boolean(state.enabled)
        startupTraySupported = Boolean(state.supported)
        if (state.reason) startupTrayNotice = state.reason
      })
      .catch(error => {
        startupTrayNotice = messageOf(error)
      })
      .finally(() => {
        startupTraySettingLoaded = true
      })
    void window.nogirem.getTurboKeySetting()
      .then(state => {
        applyTurboKeyState(state)
      })
      .catch(error => {
        turboKeyNotice = messageOf(error)
      })
      .finally(() => {
        turboKeySettingLoaded = true
      })
    void window.nogirem.getBlackboxSetting()
      .then(state => {
        applyBlackboxState(state)
      })
      .catch(() => {})
      .finally(() => {
        blackboxSettingLoaded = true
      })
    void window.nogirem.getLaunchContext()
      .catch(() => ({ startupTray: false }))
      .then(launchContext => {
        startupDataReady = true
        if (launchContext.startupTray) {
          startupAnimationFinished = true
          gameWave?.skipStartup()
        } else {
          gameWave?.allowStartup()
          finishStartupWhenReady()
        }
        void loadAll()
      })
    const removeCloseListener = window.nogirem.onCloseRequested(() => {
      closeActionPending = false
      closeModalVisible = true
    })
    const timer = window.setInterval(() => {
      void syncAffinityRuntime()
      void syncMemoryRuntime()
    }, 500)
    const turboKeyTimer = window.setInterval(() => {
      void syncTurboKeySetting()
    }, 1000)
    const blackboxTimer = window.setInterval(() => {
      void syncBlackboxSetting()
    }, 1000)
    return () => {
      window.clearInterval(timer)
      window.clearInterval(turboKeyTimer)
      window.clearInterval(blackboxTimer)
      window.clearTimeout(startupIdentityTimer)
      window.clearTimeout(leftTopContentTimer)
      window.clearTimeout(creatorNavigationTimer)
      window.clearTimeout(spinnerFinishTimer)
      stopCreatorScroll()
      document.removeEventListener("visibilitychange", syncPageVisibility)
      window.removeEventListener("keydown", handleApplicationKeydown, true)
      removeGraphicsStatusListener()
      removeVisualActivityListener()
      removeUpdateStateListener()
      removeCloseListener()
    }
  })
</script>

<svelte:head>
  <title>마비노기 렘 부스터 - {packageInfo.version}</title>
</svelte:head>

<div
  class="window-drag"
  class:creator-active={creatorViewPhase !== "home"}
  aria-hidden="true"
></div>
{#if creatorViewPhase !== "home"}
  <div class="window-drag creator-window-drag-right" aria-hidden="true"></div>
{/if}
<div class="window-controls">
  <button
    class="window-control window-minimize"
    aria-label="트레이로 최소화"
    onclick={minimizeApplicationToTray}
  >
    <svg class="control-icon-bg" width="25" height="25" aria-hidden="true">
      <line x1="0" y1="24" x2="25" y2="24" stroke-width="2" />
    </svg>
    <svg width="25" height="25" aria-hidden="true">
      <line class="control-line-one" x1="0" y1="24" x2="25" y2="24" stroke-width="2" />
    </svg>
  </button>
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

<button
  type="button"
  class="app-version"
  class:paused={visualPaused}
  aria-label="최신 업데이트 확인"
  onclick={checkApplicationUpdate}
>
  {packageInfo.version}
</button>

{#if creatorPromptVisible}
  <span
    class="creator-prompt"
    in:fly={{ y: 5, duration: 280 }}
    out:fade={{ duration: 180 }}
  >
    고급 기능은 여기에서
  </span>
{/if}

<button
  class="creator-credit entered"
  class:hidden={settingsVisible}
  class:paused={visualPaused}
  aria-label={creatorViewPhase === "home" ? "제작자 소개 열기" : "기존 화면으로 돌아가기"}
  disabled={!interfaceVisible
    || settingsVisible
    || startupIdentityPhase !== "done"
    || !creatorNavigationReady
    || creatorViewPhase === "opening"
    || creatorViewPhase === "closing"
    || Boolean(colorTransition)}
  onclick={toggleCreatorView}
>
  [류트@렘] 제작
</button>

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
          <strong>그래픽 최적화</strong>
          <span>{graphicsReady(services.graphics.data) ? "최적화됨" : "확인 필요"}</span>
        </div>
        <button
          class="setting-action"
          disabled={services.graphics.loading
            || services.graphics.optimizing
            || !services.graphics.data?.detected
            || graphicsReady(services.graphics.data)}
          onclick={requestGraphicsOptimization}
        >
          {services.graphics.optimizing ? "적용 중" : "최적화"}
        </button>
      </div>
    </section>
    {:else}
      <section
        class="boost-home"
        class:paused={visualPaused}
        class:waiting={startupIdentityPhase === "done" && displayedStatusText === "부스트 대기중"}
        class:creator-opening={creatorViewPhase === "opening"}
        class:creator-open={creatorViewPhase === "open"}
        class:creator-closing={creatorViewPhase === "closing"}
        aria-label="실시간 부스트 상태"
        onanimationend={finishCreatorViewTransition}
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
            <div class:ready={graphicsReady(services.graphics.data)}>
              {#if graphicsReady(services.graphics.data)}
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
                {services.graphics.loading
                  ? "확인 중"
                  : (graphicsReady(services.graphics.data) ? "최적화됨" : "확인 필요")}
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
          {#if blackboxFeatureEnabled}
            <div
              class="blackbox-main-controls"
              class:entered={leftTopContentEntered}
            >
            <button
              class="blackbox-main-link"
              class:active={blackboxDisplayedEnabled}
              disabled={!blackboxSettingLoaded || blackboxTogglePending}
              aria-label={blackboxEnabled ? "블랙박스 끄기" : "블랙박스 켜기"}
              aria-pressed={blackboxEnabled}
              onclick={toggleMainBlackbox}
            >
              <span
                class="blackbox-text-final"
                class:concealed={["enter", "hold"].includes(blackboxTransitionPhase)}
              >
                {blackboxSettingLoaded ? blackboxDisplayedText : "블랙박스 확인 중"}
              </span>
              {#key blackboxTransitionId}
                {#if blackboxTransitionPhase === "enter"}
                  <span
                    class="blackbox-text-base"
                    class:active-source={blackboxTransitionFromEnabled}
                  >
                    {blackboxTransitionFrom}
                  </span>
                  <span
                    class="blackbox-text-over"
                    aria-hidden="true"
                    onanimationend={holdBlackboxTransitionMask}
                  >
                    {blackboxTransitionTo}
                  </span>
                {:else if blackboxTransitionPhase === "hold"}
                  <span
                    class="blackbox-text-over holding"
                    aria-hidden="true"
                  >
                    {blackboxTransitionTo}
                  </span>
                {:else if blackboxTransitionPhase === "leave"}
                  <span
                    class="blackbox-text-over leaving"
                    aria-hidden="true"
                    onanimationend={finishBlackboxTransition}
                  >
                    {blackboxTransitionTo}
                  </span>
                {/if}
              {/key}
            </button>
            <button
              class="blackbox-window-link"
              class:active={blackboxDisplayedEnabled}
              aria-label="게임 블랙박스 관리 창 열기"
              onclick={() => window.nogirem.openBlackboxManager()}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M4 6h10a2 2 0 0 1 2 2v2.2l4-2.4a1 1 0 0 1 1.5.86v6.68a1 1 0 0 1-1.5.86l-4-2.4V16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z" />
              </svg>
            </button>
            </div>
          {/if}
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
              class:from-waiting={statusTransitionFrom === "부스트 대기중"
                && statusTransitionTo === "실시간 부스트중"}
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
        <span
          class="status-guide"
          class:waiting={displayedStatusText === "부스트 대기중"}
        >
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
      {#if creatorViewPhase !== "home"}
        <section
          class="creator-view"
          class:paused={visualPaused}
          class:opening={creatorViewPhase === "opening"}
          class:open={creatorViewPhase === "open"}
          class:closing={creatorViewPhase === "closing"}
          aria-label="제작자 및 프로그램 정보"
        >
          <button
            class="creator-return"
            aria-label="기존 기능으로 돌아가기"
            disabled={creatorViewPhase !== "open"}
            onclick={closeCreatorView}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="m5.7 8.3 6.3 6.3 6.3-6.3 1.4 1.4-7.7 7.7-7.7-7.7 1.4-1.4Z" />
            </svg>
          </button>
          <nav class="creator-tabs" aria-label="소개 항목">
            {#each creatorSections as section}
              <button
                class:active={creatorTab === section.id}
                aria-current={creatorTab === section.id ? "page" : undefined}
                disabled={creatorViewPhase !== "open" || creatorTabTransitionPhase !== "idle"}
                onclick={() => selectCreatorTab(section.id)}
              >
                {section.label}
              </button>
            {/each}
          </nav>
          <section
            class="creator-content"
            aria-label={activeCreatorSection().label}
            bind:this={creatorContentElement}
            onwheel={smoothCreatorScroll}
          >
            <div
              class="creator-content-inner"
              class:tab-leaving={creatorTabTransitionPhase === "leaving"}
              class:tab-entering={creatorTabTransitionPhase === "entering"}
              onanimationend={finishCreatorTabTransition}
            >
              {#if creatorTab === "developer"}
                <div class="creator-introduction">
                  <button
                    class="creator-channel-card"
                    aria-label="마비노기 렘 YouTube 채널 열기"
                    onclick={() => window.nogirem.openCreatorChannel()}
                  >
                    <img
                      src={creatorChannelProfile?.avatarDataUrl ?? creatorChannelAvatarUrl}
                      alt=""
                      draggable="false"
                    />
                    <span class="creator-channel-copy">
                      <strong>{creatorChannelProfile?.name ?? "마비노기 렘"}</strong>
                      {#if creatorChannelProfile}
                        <span>{creatorChannelProfile.handle}</span>
                        <small>{creatorChannelStatistics()}</small>
                      {/if}
                    </span>
                    <svg class="youtube-logo" viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M23.5 6.2a3 3 0 0 0-2.1-2.1C19.5 3.6 12 3.6 12 3.6s-7.5 0-9.4.5A3 3 0 0 0 .5 6.2 31 31 0 0 0 0 12a31 31 0 0 0 .5 5.8 3 3 0 0 0 2.1 2.1c1.9.5 9.4.5 9.4.5s7.5 0 9.4-.5a3 3 0 0 0 2.1-2.1A31 31 0 0 0 24 12a31 31 0 0 0-.5-5.8ZM9.6 15.6V8.4l6.3 3.6-6.3 3.6Z" />
                    </svg>
                  </button>
                  <div class="introduce-markdown">
                    <MarkdownBlocks blocks={introduceBlocks} onlink={openOperationPolicyLink} />
                  </div>
                </div>
              {:else if creatorTab === "operation"}
                <div class="introduce-markdown operation-markdown">
                  <MarkdownBlocks blocks={operationBlocks} onlink={openOperationPolicyLink} />
                </div>
              {:else if creatorTab === "donation"}
                <div class="donation-grid">
                  <article class="donation-card">
                    <header>
                      <img
                        src={creatorChannelProfile?.avatarDataUrl ?? creatorChannelAvatarUrl}
                        alt=""
                        draggable="false"
                      />
                      <div>
                        <small>개인 후원</small>
                        <h2>류트 서버 렘</h2>
                      </div>
                    </header>
                    <p>
                      기능에 대한 문의나 간단한 커피값 지원 등은 마비노기 류트 서버의 렘 캐릭터로 연락해 주세요.
                    </p>
                  </article>
                  <button
                    class="donation-card direct-donation-card"
                    aria-label="곧장기부 공식 페이지 열기"
                    onclick={() => window.nogirem.openDirectDonation()}
                  >
                    <header>
                      <img
                        src={directDonationLogoUrl}
                        alt=""
                        draggable="false"
                      />
                      <div>
                        <small>추천 기부처</small>
                        <h2>곧장기부</h2>
                      </div>
                    </header>
                    <p>
                      SK그룹이 지원하는 행복나눔재단에서 운영하며, 기부금이 수수료 없이 100% 전달되는 것이 특징입니다.
                    </p>
                  </button>
                </div>
              {:else if creatorTab === "history"}
                <div class="version-history">
                  {#each versionHistoryEntries as entry}
                    <article>
                      <h2>{entry.version}</h2>
                      <ul>
                        {#each entry.changes as change}
                          <li>{change}</li>
                        {/each}
                      </ul>
                    </article>
                  {/each}
                </div>
              {:else if creatorTab === "developer-tools"}
                <div class="developer-tools">
                  <div class="developer-tool-row">
                    <div>
                      <h2>버그 리포트</h2>
                      <p>문제가 발생한 경우 로그 추출 파일을 전송해 주세요</p>
                    </div>
                    <button
                      disabled={diagnosticLogAction}
                      onclick={exportDiagnosticLogs}
                    >
                      {diagnosticLogAction === "exporting" ? "압축 중…" : "로그 추출"}
                    </button>
                  </div>
                  {#if diagnosticLogNotice}
                    <span class="developer-tool-status">{diagnosticLogNotice}</span>
                  {/if}
                  <div class="developer-tool-row">
                    <div>
                      <h2>Windows 시작 시 트레이 실행</h2>
                      <p>로그인하면 창과 시작 음악 없이 백그라운드에서 실행합니다</p>
                    </div>
                    <button
                      class:active={startupTrayEnabled}
                      disabled={!startupTraySettingLoaded || !startupTraySupported || startupTrayAction}
                      aria-pressed={startupTrayEnabled}
                      onclick={toggleStartupTray}
                    >
                      {startupTrayAction === "saving"
                        ? "저장 중…"
                        : (startupTrayEnabled ? "사용 중" : "사용하기")}
                    </button>
                  </div>
                  {#if startupTrayNotice}
                    <span class="developer-tool-status">{startupTrayNotice}</span>
                  {/if}
                  <div class="developer-tool-row">
                    <div>
                      <h2>터보 키</h2>
                      <p>
                        {turboKeySettingLoaded
                          ? (turboKeyInstalled
                            ? "키를 누르고 있으면 해당 키를 반복해서 연타합니다"
                            : "기능을 사용하려면 다운로드가 필요합니다")
                          : "설치 상태를 확인하고 있습니다"}
                      </p>
                    </div>
                    <div class="developer-tool-actions">
                      {#if turboKeyInstalled}
                        <div class="turbo-key-installed-actions">
                          <div class="turbo-key-primary-actions">
                            {#if turboKeyEnabled}
                              <button
                                class="developer-tool-secondary"
                                disabled={turboKeyAction}
                                onclick={openTurboKeySettings}
                              >
                                키 설정
                              </button>
                            {/if}
                            <button
                              class:active={turboKeyEnabled}
                              disabled={turboKeyAction}
                              aria-pressed={turboKeyEnabled}
                              onclick={toggleTurboKey}
                            >
                              {turboKeyAction === "saving"
                                ? "저장 중…"
                                : (turboKeyEnabled
                                  ? (turboKeyRunning ? "사용 중" : "실행 오류")
                                  : "사용하기")}
                            </button>
                          </div>
                          <button
                            class="turbo-key-remove"
                            disabled={turboKeyAction}
                            onclick={removeTurboKey}
                          >
                            {turboKeyAction === "removing" ? "제거 중…" : "터보키 제거하기"}
                          </button>
                        </div>
                      {:else}
                        <button
                          disabled={!turboKeySettingLoaded || turboInstallAction}
                          onclick={openTurboTerms}
                        >
                          {turboKeySettingLoaded ? "다운로드" : "확인 중…"}
                        </button>
                      {/if}
                    </div>
                  </div>
                  {#if turboKeyNotice}
                    <span class="developer-tool-status">{turboKeyNotice}</span>
                  {/if}
                  <div class="developer-tool-row">
                    <div>
                      <h2>게임 블랙박스</h2>
                      <p>메인 화면에서 게임 화면 순환 녹화와 클립 저장 기능을 사용할 수 있습니다</p>
                    </div>
                    <button
                      class:active={blackboxFeatureEnabled}
                      disabled={!blackboxSettingLoaded || blackboxFeatureAction}
                      aria-pressed={blackboxFeatureEnabled}
                      onclick={toggleBlackboxFeature}
                    >
                      {blackboxFeatureAction === "saving"
                        ? "저장 중…"
                        : (blackboxFeatureEnabled ? "사용 중" : "사용하기")}
                    </button>
                  </div>
                  {#if blackboxFeatureNotice}
                    <span class="developer-tool-status">{blackboxFeatureNotice}</span>
                  {/if}
                  <div class="developer-tool-stack">
                    <div>
                      <h2>마비노기 CPU 우선 점유 비율 설정</h2>
                      <p>
                        {services.affinity.data?.gameCoreSetting?.hybrid
                          ? "마비노기에 우선 배정할 P코어 개수를 선택합니다"
                          : "마비노기에 우선 배정할 물리 코어 개수를 선택합니다"}
                      </p>
                    </div>
                    {#if gameCpuCoreOptions().length}
                      <div class="game-cpu-core-controls" aria-label="마비노기 CPU 코어 개수">
                        {#each gameCpuCoreOptions() as coreCount}
                          <button
                            class="game-cpu-core-option"
                            class:allocated={services.affinity.data?.gameCoreSetting?.gameCoreCount
                              >= coreCount}
                            disabled={gameCpuCoreAction
                              || services.affinity.data?.cpuReorder?.state === "running"}
                            aria-pressed={services.affinity.data?.gameCoreSetting?.gameCoreCount
                              >= coreCount}
                            style={`--allocation-color: ${gameCpuCoreColor(coreCount)}`}
                            onclick={() => selectGameCpuCoreCount(coreCount)}
                          >
                            {coreCount}
                          </button>
                        {/each}
                        {#if gameCpuUnavailableCores().length}
                          <span class="game-cpu-unavailable-label">선택 불가 코어</span>
                          {#each gameCpuUnavailableCores() as core}
                            <button
                              class="game-cpu-unavailable-core"
                              disabled
                              title={core.reason}
                            >
                              {core.label}
                            </button>
                          {/each}
                        {/if}
                      </div>
                    {:else}
                      <span class="developer-tool-status">
                        물리 CPU 코어 구성을 확인할 수 없습니다
                      </span>
                    {/if}
                  </div>
                  {#if gameCpuCoreNotice}
                    <span class="developer-tool-status">{gameCpuCoreNotice}</span>
                  {/if}
                  <div class="developer-tool-row">
                    <div>
                      <h2>CPU 재정렬</h2>
                      <p>CPU 격리 구성을 재설정하여 캐시 초기화를 유도합니다</p>
                    </div>
                    <button
                      disabled={!cpuReorderAvailable()}
                      onclick={runCpuReorder}
                    >
                      {cpuReorderAction === "running" ? "재정렬 중…" : "CPU 재정렬"}
                    </button>
                  </div>
                  {#if cpuReorderNotice}
                    <span class="developer-tool-status">{cpuReorderNotice}</span>
                  {/if}
                </div>
              {:else}
                {#each activeCreatorSection().content as paragraph}
                  <p>{paragraph}</p>
                {/each}
              {/if}
            </div>
          </section>
        </section>
      {/if}
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
          <h2>{services.graphics.data?.title ?? "그래픽 설정"}</h2>
        </div>
        {#if services.graphics.loading}
          <span class="status checking">확인 중</span>
        {:else if services.graphics.error}
          <span class="status error">확인 실패</span>
        {:else if graphicsReady(services.graphics.data)}
          <span class="status ready">최적화됨</span>
        {:else}
          <span class="status needed">최적화 필요</span>
        {/if}
      </div>

      {#if services.graphics.error}
        <p class="error-message">{services.graphics.error}</p>
      {:else if services.graphics.data}
        {#if services.graphics.data.detected}
          <p class="device">
            {services.graphics.data.gpus?.map(gpu => [gpu.name, gpu.driverVersion].filter(Boolean).join(" · ")).join(", ")}
          </p>
          <dl class="checks">
            {#each services.graphics.data.goalsList ?? [] as goal}
              <div>
                <dt>{goal.label}</dt>
                <dd class:passed={goal.met}>
                  {goal.supported === false
                    ? "해당 없음"
                    : (goal.error ? "적용 실패" : (goal.met ? "완료" : "필요"))}
                </dd>
              </div>
            {/each}
          </dl>
          <p class="meta">
            적용 범위: {services.graphics.data.scopeLabel ?? "확인되지 않음"}
          </p>
        {:else}
          <p class="empty">{services.graphics.data.reason ?? "지원되는 GPU를 찾지 못했습니다"}</p>
        {/if}
      {:else}
        <div class="skeleton"></div>
      {/if}

      <div class="actions">
        <button
          class="secondary compact"
          disabled={services.graphics.loading || services.graphics.optimizing}
          onclick={() => refreshService("graphics", window.nogirem.refreshGraphics)}
        >
          다시 확인
        </button>
        <button
          class="primary"
          disabled={services.graphics.loading
            || services.graphics.optimizing
            || !services.graphics.data?.detected
            || graphicsReady(services.graphics.data)}
          onclick={requestGraphicsOptimization}
        >
          {services.graphics.optimizing ? "적용 중…" : "그래픽 최적화"}
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
          {services.network.data.fastPing.supported === false
            ? services.network.data.fastPing.reason
            : `${services.network.data.fastPing.current?.interfaceAlias} #${services.network.data.fastPing.current?.interfaceIndex}`}
        </p>
        <dl class="checks network-checks">
          <div class="network-checks-heading">
            <dt>옵션</dt>
            <dd>현재값</dd>
            <dd>권장값</dd>
          </div>
          <div>
            <dt>TCP ACK 빈도</dt>
            <dd class:passed={services.network.data.fastPing.supported === false
              || services.network.data.fastPing.current?.TcpAckFrequency === 1}>
              {services.network.data.fastPing.supported === false
                ? "적용 생략"
                : (services.network.data.fastPing.current?.TcpAckFrequency ?? "없음")}
            </dd>
            <dd class="passed">1</dd>
          </div>
          <div>
            <dt>TCP No Delay</dt>
            <dd class:passed={services.network.data.fastPing.supported === false
              || services.network.data.fastPing.current?.TCPNoDelay === 1}>
              {services.network.data.fastPing.supported === false
                ? "적용 생략"
                : (services.network.data.fastPing.current?.TCPNoDelay ?? "없음")}
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

{#if turboTermsModalVisible}
  <TermsModal
    title="터보 키 다운로드 전 확인"
    closeSignal={turboTermsModalCloseSignal}
    closeDisabled={Boolean(turboInstallAction)}
    onclose={() => turboTermsModalVisible = false}
  >
    <div class="introduce-markdown turbo-terms-content">
      <MarkdownBlocks blocks={turboTermsBlocks} onlink={openOperationPolicyLink} />
    </div>
    {#snippet footer()}
      <div class="turbo-terms-footer">
        {#if turboKeyNotice}
          <span class="turbo-terms-error">{turboKeyNotice}</span>
        {/if}
        <div class="turbo-terms-actions">
          <button
            class="developer-tool-secondary"
            disabled={Boolean(turboInstallAction)}
            onclick={() => turboTermsModalCloseSignal++}
          >
            취소
          </button>
          <button
            disabled={Boolean(turboInstallAction)}
            onclick={downloadTurboKey}
          >
            {turboInstallAction ? "다운로드 중…" : "확인 후 다운로드"}
          </button>
        </div>
      </div>
    {/snippet}
  </TermsModal>
{/if}

{#if turboKeyModalVisible}
  <Modal
    title="터보 키 적용 대상 설정"
    variant="fullscreen"
    closeSignal={turboKeyModalCloseSignal}
    closeDisabled={turboKeyAction === "keys"}
    onclose={closeTurboKeySettings}
  >
    <div class="turbo-key-picker">
      <div class="turbo-key-picker-summary">
        <p>화면의 키를 클릭하거나 실제 키보드 키를 눌러 선택 또는 해제하세요</p>
      </div>
      <div class="turbo-keyboard" aria-label="터보 키 선택용 키보드">
        <div class="turbo-keyboard-function-row">
          {#each functionKeyboardRow as key}
            {#if key.spacer}
              <span
                class="turbo-key-spacer"
                style={`--key-units: ${key.spacer}`}
                aria-hidden="true"
              ></span>
            {:else}
              <button
                class="turbo-keycap"
                class:group-start={key.groupStart}
                class:selected={key.code && turboKeyDraftCodes.includes(key.code)}
                style={`--key-units: ${key.units ?? 1}`}
                disabled={key.disabled}
                aria-pressed={key.code ? turboKeyDraftCodes.includes(key.code) : undefined}
                onclick={() => key.code && toggleTurboKeyCode(key.code)}
              >
                {key.label}
              </button>
            {/if}
          {/each}
        </div>
        <div class="turbo-keyboard-body">
          <div class="turbo-keyboard-main">
            {#each mainKeyboardRows as row}
              <div class="turbo-keyboard-row">
                {#each row as key}
                  <button
                    class="turbo-keycap"
                    class:selected={key.code && turboKeyDraftCodes.includes(key.code)}
                    style={`--key-units: ${key.units ?? 1}`}
                    disabled={key.disabled}
                    aria-pressed={key.code ? turboKeyDraftCodes.includes(key.code) : undefined}
                    onclick={() => key.code && toggleTurboKeyCode(key.code)}
                  >
                    {key.label}
                  </button>
                {/each}
              </div>
            {/each}
          </div>
          <div class="turbo-keyboard-navigation">
            {#each navigationKeys as key}
              <button
                class="turbo-keycap"
                class:selected={turboKeyDraftCodes.includes(key.code)}
                style={`grid-row: ${key.row}; grid-column: ${key.column}`}
                aria-pressed={turboKeyDraftCodes.includes(key.code)}
                onclick={() => toggleTurboKeyCode(key.code)}
              >
                {key.label}
              </button>
            {/each}
          </div>
          <div class="turbo-keyboard-numpad">
            {#each numpadKeys as key}
              <button
                class="turbo-keycap"
                class:selected={key.code && turboKeyDraftCodes.includes(key.code)}
                style={`grid-row: ${key.row} / span ${key.rowSpan ?? 1}; grid-column: ${key.column} / span ${key.columnSpan ?? 1}`}
                disabled={key.disabled}
                aria-pressed={key.code ? turboKeyDraftCodes.includes(key.code) : undefined}
                onclick={() => key.code && toggleTurboKeyCode(key.code)}
              >
                {key.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
      <div class="turbo-key-picker-actions">
        <label class="turbo-key-interval">
          <span>입력 간격</span>
          <select bind:value={turboKeyDraftIntervalMs}>
            {#each turboKeyIntervalOptions as interval}
              <option value={interval}>{interval} ms</option>
            {/each}
          </select>
          <span class="turbo-key-interval-hint">저사양 PC에서는 입력 간격을 늘리세요</span>
        </label>
        <div class="turbo-key-picker-buttons">
          <button
            class="reset"
            onclick={resetTurboKeyCodes}
          >
            초기화
          </button>
          <button
            onclick={saveTurboKeyCodes}
          >
            설정 완료
          </button>
        </div>
      </div>
    </div>
  </Modal>
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
    closeSignal={optimizationModalCloseSignal}
    onclose={() => optimizationModalVisible = false}
  >
    <div class="optimization-modal-grid">
      <section class="optimization-detail" aria-labelledby="graphics-status-title">
        <header>
          <div class="detail-heading-copy">
            <span class="detail-category">그래픽 설정</span>
            <h3 id="graphics-status-title">
              {services.graphics.data?.title ?? "그래픽 설정"}
            </h3>
          </div>
          <button
            class="detail-action"
            class:complete={graphicsReady(services.graphics.data)}
            disabled={services.graphics.loading
              || services.graphics.optimizing
              || !services.graphics.data?.detected
              || graphicsReady(services.graphics.data)}
            onclick={requestGraphicsOptimization}
          >
            {services.graphics.loading
              ? "확인 중"
              : (services.graphics.optimizing
                ? "적용 중"
                : (graphicsReady(services.graphics.data) ? "완료됨" : "최적화"))}
          </button>
        </header>
        {#if services.graphics.error}
          <p class="detail-error">{services.graphics.error}</p>
        {:else if services.graphics.data?.detected}
          <p class="detail-device">
            {services.graphics.data.gpus?.map(gpu => gpu.name).join(", ")}
          </p>
          <dl class="detail-list">
            {#each services.graphics.data.goalsList ?? [] as goal}
              <div>
                <dt>{goal.label}</dt>
                <dd class:ready={goal.met}>
                  {goal.supported === false
                    ? "해당 없음"
                    : (goal.error ? "적용 실패" : (goal.met ? "완료" : "조정 필요"))}
                </dd>
              </div>
            {/each}
          </dl>
          {#each (services.graphics.data.goalsList ?? []).filter(goal => goal.error) as goal}
            <p class="detail-error">{goal.error}</p>
          {/each}
        {:else}
          <p class="detail-empty">
            {services.graphics.loading
              ? "그래픽 설정을 확인하고 있습니다"
              : (services.graphics.data?.reason ?? "지원되는 GPU를 찾지 못했습니다")}
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
          <div class="detail-device-row">
            <p class="detail-device">
              {services.network.data.fastPing.supported === false
                ? services.network.data.fastPing.reason
                : (services.network.data.fastPing.current?.interfaceAlias ?? "기본 네트워크")}
            </p>
            <button
              class="detail-restore"
              disabled={services.network.loading
                || services.network.optimizing
                || !networkRestoreAvailable(services.network.data)}
              onclick={requestNetworkRestore}
            >
              {services.network.optimizing && networkReconnectMode === "restore"
                ? "되돌리는 중"
                : "설정 되돌리기"}
            </button>
          </div>
          <dl class="detail-list">
            <div>
              <dt>TCP ACK 빈도</dt>
              <dd class:ready={services.network.data.fastPing.supported === false
                || services.network.data.fastPing.current?.TcpAckFrequency === 1}>
                {services.network.data.fastPing.supported === false
                  ? "적용 생략"
                  : (services.network.data.fastPing.current?.TcpAckFrequency === 1 ? "완료" : "조정 필요")}
              </dd>
            </div>
            <div>
              <dt>TCP No Delay</dt>
              <dd class:ready={services.network.data.fastPing.supported === false
                || services.network.data.fastPing.current?.TCPNoDelay === 1}>
                {services.network.data.fastPing.supported === false
                  ? "적용 생략"
                  : (services.network.data.fastPing.current?.TCPNoDelay === 1 ? "완료" : "조정 필요")}
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

{#if radeonGlobalModalVisible}
  <Modal
    eyebrow="AMD Radeon 최적화"
    title="Radeon 전역 설정을 변경합니다"
    closeSignal={radeonGlobalCloseSignal}
    onclose={finishRadeonGlobalModal}
  >
    <p class="modal-description">
      수직 동기화, Enhanced Sync, Anti-Lag, Chill 설정이 Radeon GPU 전역에 적용되며
      다른 게임에서도 유지됩니다. 최대 프레임 제한은 변경하지 않습니다.
    </p>
    <div class="modal-actions">
      <button
        class="secondary"
        disabled={Boolean(radeonGlobalAction)}
        onclick={() => closeRadeonGlobalModal("cancel")}
      >
        취소
      </button>
      <button
        class="monochrome"
        disabled={Boolean(radeonGlobalAction)}
        onclick={() => closeRadeonGlobalModal("continue")}
      >
        계속하기
      </button>
    </div>
  </Modal>
{/if}

{#if networkReconnectModalVisible}
  <Modal
    eyebrow={networkReconnectMode === "restore" ? "네트워크 설정 복원" : "네트워크 최적화"}
    title={networkReconnectMode === "restore"
      ? "패스트핑 설정을 되돌립니다"
      : "네트워크 연결을 다시 시작합니다"}
    hideClose={true}
    closeSignal={networkReconnectCloseSignal}
    onclose={finishNetworkReconnectModal}
  >
    <p class="modal-description">
      {networkReconnectMode === "restore"
        ? "TCP ACK 빈도와 TCP No Delay를 이전 설정으로 되돌립니다. 기록이 없으면 두 옵션을 끄며 TCP 자동 조정은 유지합니다."
        : "TCP ACK 빈도 또는 TCP No Delay를 적용하려면 네트워크 어댑터를 다시 연결해야 합니다."}
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
        {networkReconnectMode === "restore" ? "되돌리기" : "계속하기"}
      </button>
    </div>
  </Modal>
{/if}

{#if closeModalVisible}
  <Modal
    eyebrow="프로그램 종료"
    title="프레임 부스트를 정지할까요?"
    closeSignal={closeModalCloseSignal}
    closeDisabled={closeActionPending}
    onclose={() => confirmClose("cancel")}
  >
    <p class="modal-description">
      부스트를 유지한 채로 종료하면 마비노기 외 프로그램들의 성능이 제한될 수 있습니다.
    </p>
    <div class="modal-actions">
      <button
        class="secondary"
        disabled={closeActionPending}
        onclick={() => confirmClose("keep")}
      >
        적용 유지 후 종료
      </button>
      <button
        class="primary"
        disabled={closeActionPending}
        onclick={() => confirmClose("reset")}
      >
        {closeActionPending ? "종료 준비 중…" : "부스트 설정 되돌린 후 종료"}
      </button>
    </div>
  </Modal>
{/if}

{#if !closeModalVisible && (
  applicationUpdateState.phase === "downloading"
  || applicationUpdateState.phase === "downloaded"
)}
  <UpdatePreviewModal
    progress={applicationUpdateState.percent}
    downloaded={applicationUpdateState.phase === "downloaded"}
    installing={applicationUpdateInstalling}
    onInstall={installApplicationUpdate}
  />
{/if}
