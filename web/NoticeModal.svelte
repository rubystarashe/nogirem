<script>
  import { onDestroy } from "svelte"

  let {
    title,
    closeSignal = 0,
    onconfirm = () => {},
    children,
  } = $props()
  let closing = $state(false)
  let closeTimer
  let observedCloseSignal = $state()
  let scrollElement
  let scrollTarget = 0
  let scrollFrame

  function confirm() {
    if (closing) return
    closing = true
    closeTimer = window.setTimeout(onconfirm, 180)
  }

  function stopSmoothScroll() {
    if (scrollFrame) cancelAnimationFrame(scrollFrame)
    scrollFrame = null
  }

  function animateSmoothScroll() {
    if (!scrollElement) {
      scrollFrame = null
      return
    }
    const distance = scrollTarget - scrollElement.scrollTop
    if (Math.abs(distance) < 0.5) {
      scrollElement.scrollTop = scrollTarget
      scrollFrame = null
      return
    }
    scrollElement.scrollTop += distance * 0.18
    scrollFrame = requestAnimationFrame(animateSmoothScroll)
  }

  function smoothScroll(event) {
    if (event.ctrlKey || !scrollElement) return
    const maximum = scrollElement.scrollHeight - scrollElement.clientHeight
    if (maximum <= 0) return
    const unit = event.deltaMode === 1
      ? 16
      : event.deltaMode === 2
        ? scrollElement.clientHeight
        : 1
    const start = scrollFrame ? scrollTarget : scrollElement.scrollTop
    const next = Math.max(0, Math.min(maximum, start + event.deltaY * unit * 0.8))
    if (next === start) return
    event.preventDefault()
    scrollTarget = next
    if (!scrollFrame) scrollFrame = requestAnimationFrame(animateSmoothScroll)
  }

  $effect(() => {
    if (observedCloseSignal === undefined) {
      observedCloseSignal = closeSignal
      return
    }
    if (closeSignal === observedCloseSignal) return
    observedCloseSignal = closeSignal
    confirm()
  })

  onDestroy(() => {
    window.clearTimeout(closeTimer)
    stopSmoothScroll()
  })
</script>

<div class="notice-backdrop" class:closing>
  <div
    class="notice-dialog"
    class:closing
    role="alertdialog"
    aria-modal="true"
    aria-label={title}
  >
    <div
      class="notice-content"
      bind:this={scrollElement}
      onwheel={smoothScroll}
    >
      {@render children?.()}
    </div>
  </div>
</div>

<style>
  .notice-backdrop {
    position: fixed;
    z-index: 500;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(5, 6, 7, 0.52);
    animation: notice-backdrop-in 180ms ease-out both;
    -webkit-app-region: no-drag;
  }

  .notice-backdrop.closing {
    pointer-events: none;
    animation: notice-backdrop-out 180ms ease-in both;
  }

  .notice-dialog {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    width: min(100%, 540px);
    max-height: calc(100vh - 48px);
    padding: 14px 16px;
    overflow: hidden;
    border: 1px solid rgba(0, 0, 0, 0.08);
    border-radius: 14px;
    color: #17191b;
    background: #fff;
    box-shadow: 0 22px 65px rgba(0, 0, 0, 0.34);
    animation: notice-dialog-in 200ms ease-out both;
  }

  .notice-dialog.closing {
    animation: notice-dialog-out 180ms ease-in both;
  }

  .notice-content {
    min-height: 0;
    margin-right: -12px;
    padding-right: 12px;
    overflow: auto;
    color: #53575b;
    font-size: 0.84rem;
    line-height: 1.6;
    scrollbar-color: rgba(20, 20, 20, 0.32) transparent;
    scrollbar-width: thin;
  }

  .notice-content :global(p) {
    margin: 0;
  }

  .notice-content::-webkit-scrollbar {
    width: 3px;
  }

  .notice-content::-webkit-scrollbar-track {
    background:
      linear-gradient(
        to right,
        transparent 1px,
        rgba(20, 20, 20, 0.14) 1px,
        rgba(20, 20, 20, 0.14) 2px,
        transparent 2px
      );
  }

  .notice-content::-webkit-scrollbar-thumb {
    border-radius: 2px;
    background: rgba(20, 20, 20, 0.32);
  }

  @keyframes notice-backdrop-in {
    from {
      background: rgba(5, 6, 7, 0);
    }

    to {
      background: rgba(5, 6, 7, 0.52);
    }
  }

  @keyframes notice-backdrop-out {
    from {
      background: rgba(5, 6, 7, 0.52);
    }

    to {
      background: rgba(5, 6, 7, 0);
    }
  }

  @keyframes notice-dialog-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }

    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes notice-dialog-out {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }

    to {
      opacity: 0;
      transform: translateY(6px) scale(0.985);
    }
  }
</style>
