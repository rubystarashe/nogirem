<script>
  import { onDestroy } from "svelte"

  let {
    title,
    confirmLabel = "확인",
    onconfirm = () => {},
    children,
  } = $props()
  let closing = $state(false)
  let closeTimer

  function confirm() {
    if (closing) return
    closing = true
    closeTimer = window.setTimeout(onconfirm, 180)
  }

  onDestroy(() => window.clearTimeout(closeTimer))
</script>

<div class="notice-backdrop" class:closing>
  <div
    class="notice-dialog"
    class:closing
    role="alertdialog"
    aria-modal="true"
    aria-label={title}
  >
    <h2>{title}</h2>
    <div class="notice-content">
      {@render children?.()}
    </div>
    <div class="notice-actions">
      <button type="button" disabled={closing} onclick={confirm}>
        {confirmLabel}
      </button>
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
    width: min(100%, 540px);
    padding: 24px 26px 20px;
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

  h2 {
    margin: 0 0 12px;
    font-size: 1.18rem;
    line-height: 1.35;
  }

  .notice-content {
    color: #53575b;
    font-size: 0.84rem;
    line-height: 1.6;
  }

  .notice-content :global(p) {
    margin: 0;
  }

  .notice-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 22px;
  }

  button {
    min-width: 78px;
    height: 36px;
    padding: 0 18px;
    border: 0;
    border-radius: 7px;
    color: #fff;
    background: #17191b;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 700;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: #303337;
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
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
