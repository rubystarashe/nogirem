<script>
  export let progress = 0
  export let downloaded = false
  export let installing = false
  export let onInstall = () => {}

  $: visibleProgress = Math.max(0, Math.min(100, Number(progress) || 0))
</script>

<div class="update-preview-overlay" role="dialog" aria-modal="true" aria-label="업데이트">
  <section class="update-preview-panel">
    <h2>
      {downloaded ? "새 버전 다운로드 완료됨" : "새 버전을 가져오고 있습니다"}
    </h2>

    {#if !downloaded}
      <div class="update-progress" aria-live="polite">
        <span>{Math.floor(visibleProgress)}</span>
        <div class="update-progress-track">
          <div class="update-progress-value" style={`width: ${visibleProgress}%`}></div>
        </div>
      </div>
    {:else}
      <button type="button" disabled={installing} onclick={onInstall}>
        {installing ? "설치 준비 중" : "새 버전 설치"}
      </button>
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
    min-width: 168px;
    height: 48px;
    padding: 0 18px;
    border: 1px solid #fff;
    border-radius: 0;
    color: #000;
    background: #fff;
    font-size: 16.5px;
    font-weight: 700;
    cursor: pointer;
    animation: install-button-in 220ms ease-out both;
    transition:
      color 180ms ease,
      background-color 180ms ease;
  }

  button:hover {
    color: #fff;
    background-color: transparent;
  }

  button:disabled {
    cursor: wait;
    opacity: 0.7;
  }

  .update-progress {
    position: absolute;
    right: 0;
    bottom: 20px;
    left: 0;
  }

  .update-progress > span {
    display: block;
    margin: 0 15px 0 0;
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
    transition: width 300ms ease-out;
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
