<script>
  import { onDestroy } from "svelte"
  import { createSmoothWheelScroller } from "./smooth-wheel-scroll.mjs"

  let {
    title,
    closeSignal = 0,
    closeDisabled = false,
    onclose = () => {},
    children,
    footer,
  } = $props()
  let closing = $state(false)
  let closeTimer
  let observedCloseSignal = $state()
  let scrollElement
  const scroller = createSmoothWheelScroller(() => scrollElement)

  function requestClose() {
    if (closeDisabled || closing) return
    closing = true
    closeTimer = window.setTimeout(onclose, 180)
  }

  $effect(() => {
    if (observedCloseSignal === undefined) {
      observedCloseSignal = closeSignal
      return
    }
    if (closeSignal === observedCloseSignal) return
    observedCloseSignal = closeSignal
    requestClose()
  })

  onDestroy(() => {
    window.clearTimeout(closeTimer)
    scroller.stop()
  })
</script>

<div class="terms-backdrop" class:closing>
  <div
    class="terms-modal"
    class:closing
    role="dialog"
    aria-modal="true"
    aria-labelledby="terms-modal-title"
  >
    <header>
      <h2 id="terms-modal-title">{title}</h2>
      <button
        class="terms-close"
        aria-label="약관 닫기"
        disabled={closeDisabled || closing}
        onclick={requestClose}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5.3 4 12 10.7 18.7 4 20 5.3 13.3 12l6.7 6.7-1.3 1.3-6.7-6.7L5.3 20 4 18.7l6.7-6.7L4 5.3 5.3 4Z" />
        </svg>
      </button>
    </header>
    <div
      class="terms-scroll"
      bind:this={scrollElement}
      onwheel={scroller.handleWheel}
    >
      {@render children?.()}
    </div>
    {#if footer}
      <footer>
        {@render footer()}
      </footer>
    {/if}
  </div>
</div>

<style>
  .terms-backdrop {
    position: fixed;
    z-index: 450;
    inset: 0;
    background: #191c1e;
    animation: terms-in 180ms ease-out both;
  }

  .terms-backdrop.closing {
    pointer-events: none;
    animation: terms-out 180ms ease-in both;
  }

  .terms-modal {
    display: grid;
    width: 100%;
    height: 100%;
    grid-template-rows: auto minmax(0, 1fr) auto;
    color: #f3f5f7;
    background: #191c1e;
  }

  header {
    position: relative;
    min-height: 48px;
    margin: 0;
    padding: 14px 52px 10px 18px;
    border-bottom: 1px solid #343a3d;
  }

  h2 {
    margin: 0;
    font-size: 18px;
    line-height: 24px;
  }

  .terms-close {
    position: absolute;
    top: 10px;
    right: 12px;
    width: 30px;
    height: 30px;
    padding: 5px;
    color: #aeb5b9;
    background: transparent;
    cursor: pointer;
  }

  .terms-close:hover:not(:disabled) {
    color: #fff;
  }

  .terms-close svg {
    width: 100%;
    height: 100%;
    fill: currentColor;
  }

  .terms-scroll {
    min-height: 0;
    padding: 8px 20px 18px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  footer {
    min-height: 52px;
    padding: 10px 16px;
    border-top: 1px solid #343a3d;
    background: #151719;
  }

  @keyframes terms-in {
    from {
      opacity: 0;
    }

    to {
      opacity: 1;
    }
  }

  @keyframes terms-out {
    from {
      opacity: 1;
    }

    to {
      opacity: 0;
    }
  }
</style>
