<script>
  import { onDestroy } from "svelte"

  let {
    eyebrow = "",
    title,
    variant = "sheet",
    hideTitle = false,
    closeSignal = 0,
    closeDisabled = false,
    hideClose = false,
    onclose = () => {},
    children,
  } = $props()
  let closing = $state(false)
  let closeTimer
  let observedCloseSignal = $state()

  function requestClose() {
    if (closeDisabled || closing) return
    closing = true
    closeTimer = window.setTimeout(onclose, 200)
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

  onDestroy(() => window.clearTimeout(closeTimer))
</script>

<div
  class="modal-backdrop"
  class:closing
  class:large={variant === "large"}
  class:fullscreen={variant === "fullscreen"}
>
  <div
    class="modal-sheet"
    class:closing
    class:large={variant === "large"}
    class:fullscreen={variant === "fullscreen"}
    role="dialog"
    aria-modal="true"
    aria-label={title}
  >
    <img
      class="modal-watermark"
      src="./logo2-white-transparent.png"
      alt=""
      draggable="false"
    />
    {#if !hideClose}
      <button
        class="modal-close"
        aria-label="모달 닫기"
        disabled={closeDisabled || closing}
        onclick={requestClose}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5.3 4 12 10.7 18.7 4 20 5.3 13.3 12l6.7 6.7-1.3 1.3-6.7-6.7L5.3 20 4 18.7l6.7-6.7L4 5.3 5.3 4Z" />
        </svg>
      </button>
    {/if}
    {#if eyebrow}
      <p class="modal-eyebrow">{eyebrow}</p>
    {/if}
    {#if !hideTitle}
      <h2>{title}</h2>
    {/if}
    <div class="modal-content">
      {@render children?.()}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    z-index: 600;
    inset: 0;
    display: flex;
    align-items: flex-end;
    background: rgba(7, 8, 9, 0.68);
    animation: modal-backdrop-in 180ms ease-out both;
    -webkit-app-region: no-drag;
  }

  .modal-backdrop.closing {
    pointer-events: none;
    animation: modal-backdrop-out 200ms ease-in both;
  }

  .modal-backdrop.large {
    padding-top: 30px;
    -webkit-app-region: drag;
  }

  .modal-backdrop.fullscreen {
    align-items: stretch;
    background: #191c1e;
  }

  .modal-sheet {
    position: relative;
    width: 100%;
    max-height: calc(100vh - 20px);
    padding: 22px 28px 18px;
    overflow: auto;
    border: 1px solid #383e41;
    border-bottom: 0;
    border-radius: 18px 18px 0 0;
    background: #191c1e;
    box-shadow: 0 -18px 55px rgba(0, 0, 0, 0.42);
    animation: modal-sheet-in 220ms ease-out both;
  }

  .modal-sheet.closing {
    animation: modal-sheet-out 200ms ease-in both;
  }

  .modal-watermark {
    position: absolute;
    z-index: 0;
    top: -18px;
    left: 50%;
    width: 110px;
    height: 110px;
    opacity: 0.08;
    object-fit: contain;
    pointer-events: none;
    transform: translateX(-50%);
    user-select: none;
    -webkit-user-drag: none;
  }

  .modal-close,
  .modal-eyebrow,
  h2,
  .modal-content {
    position: relative;
    z-index: 1;
  }

  .modal-sheet.large {
    min-height: 0;
    max-height: calc(100vh - 30px);
    padding: 16px 20px 19px;
    border-radius: 14px 14px 0 0;
    -webkit-app-region: no-drag;
  }

  .modal-sheet.fullscreen {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    width: 100%;
    height: 100%;
    max-height: none;
    padding: 12px 12px 10px;
    overflow: hidden;
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }

  .modal-sheet.fullscreen h2 {
    margin-bottom: 8px;
    font-size: 1.2rem;
  }

  .modal-sheet.fullscreen .modal-content {
    min-height: 0;
  }

  .modal-sheet.fullscreen .modal-close {
    top: 10px;
    right: 10px;
  }

  .modal-sheet.large .modal-eyebrow {
    display: flex;
    min-height: 24px;
    align-items: center;
  }

  .modal-close {
    position: absolute;
    z-index: 2;
    top: 14px;
    right: 18px;
    width: 28px;
    height: 28px;
    padding: 4px;
    color: #aeb5b9;
    background: transparent;
    cursor: pointer;
  }

  .modal-close:hover:not(:disabled) {
    color: #fff;
  }

  .modal-close svg {
    width: 100%;
    height: 100%;
    fill: currentColor;
  }

  .modal-eyebrow {
    margin: 0 0 8px;
    color: #8f989d;
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.14em;
  }

  h2 {
    margin: 0 44px 12px 0;
    color: #f3f5f7;
    font-size: 1.35rem;
  }

  @keyframes modal-sheet-in {
    from {
      opacity: 0;
      transform: translateY(24px);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes modal-sheet-out {
    from {
      opacity: 1;
      transform: translateY(0);
    }

    to {
      opacity: 0;
      transform: translateY(24px);
    }
  }

  @keyframes modal-backdrop-in {
    from {
      background: rgba(7, 8, 9, 0);
    }

    to {
      background: rgba(7, 8, 9, 0.68);
    }
  }

  @keyframes modal-backdrop-out {
    from {
      background: rgba(7, 8, 9, 0.68);
    }

    to {
      background: rgba(7, 8, 9, 0);
    }
  }
</style>
