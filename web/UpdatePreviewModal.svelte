<script>
  import { onMount } from "svelte"

  export let active = false

  let progress = 0
  let mounted = false
  let started = false
  let frame

  function startPreview() {
    if (!mounted || started || !active) return

    started = true
    const startedAt = performance.now()
    const duration = 15000

    const updateProgress = now => {
      progress = Math.min(100, Math.floor(((now - startedAt) / duration) * 100))
      if (progress < 100) frame = requestAnimationFrame(updateProgress)
    }

    frame = requestAnimationFrame(updateProgress)
  }

  $: if (active && mounted) startPreview()

  onMount(() => {
    mounted = true
    startPreview()

    return () => {
      if (frame) cancelAnimationFrame(frame)
    }
  })
</script>

<div class="update-preview-overlay" role="dialog" aria-modal="true" aria-label="업데이트">
  <section class="update-preview-panel">
    <h2>
      {progress < 100 ? "새 버전을 가져오고 있습니다" : "업데이트 다운로드 완료"}
    </h2>

    {#if progress < 100}
      <div class="update-progress" aria-live="polite">
        <span>{progress}</span>
        <div class="update-progress-track">
          <div class="update-progress-value" style={`width: ${progress}%`}></div>
        </div>
      </div>
    {:else}
      <button type="button" onclick={() => {}}>업데이트 설치</button>
    {/if}
  </section>
</div>

<style>
  .update-preview-overlay {
    position: fixed;
    z-index: 500;
    inset: 0;
    background: transparent;
  }

  .update-preview-panel {
    position: absolute;
    top: 64px;
    right: 0;
    left: 0;
    height: 156px;
    padding: 17px 20px 20px;
    color: #fff;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(2px);
  }

  h2 {
    margin: 0;
    color: #fff;
    font-size: 30px;
    font-weight: 700;
    line-height: 1.35;
    letter-spacing: -0.025em;
  }

  button {
    position: absolute;
    right: 20px;
    bottom: 20px;
    min-width: 112px;
    height: 32px;
    padding: 0 15px;
    border: 1px solid #fff;
    border-radius: 0;
    color: #000;
    background: #fff;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    animation: install-button-in 220ms ease-out both;
  }

  .update-progress {
    position: absolute;
    right: 0;
    bottom: 20px;
    left: 0;
  }

  .update-progress > span {
    display: block;
    margin: 0 15px 40px 0;
    color: #fff;
    font-size: 80px;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    text-align: right;
  }

  .update-progress-track {
    width: 100%;
    height: 10px;
    overflow: hidden;
    background: transparent;
  }

  .update-progress-value {
    height: 100%;
    background: #fff;
    transition: width 80ms linear;
  }

  @keyframes install-button-in {
    from {
      opacity: 0;
      transform: translateY(5px);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
