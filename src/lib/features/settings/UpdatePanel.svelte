<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";

  let {
    phase = "idle",
    message = "",
    onCheck,
  }: {
    phase?: "idle" | "checking" | "downloading" | "installing";
    message?: string;
    onCheck: () => void;
  } = $props();

  const busy = $derived(phase !== "idle");
  const buttonLabel = $derived(
    phase === "checking"
      ? "Checking"
      : phase === "downloading"
        ? "Downloading"
        : phase === "installing"
          ? "Installing"
          : "Check for Updates",
  );
</script>

<section class="update-panel" aria-labelledby="update-panel-title" aria-busy={busy}>
  <div class="update-copy">
    <div class="title-row">
      <span class:active={busy} class="status-dot" aria-hidden="true"></span>
      <h3 id="update-panel-title">Updates</h3>
    </div>
    <p>Updates are checked only when you press this button.</p>
    {#if message}
      <p class="update-status" aria-live="polite">{message}</p>
    {/if}
  </div>
  <UiButton size="small" disabled={busy} onclick={onCheck}>
    {buttonLabel}
  </UiButton>
</section>

<style>
  .update-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-width: 0;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .update-copy { min-width: 0; }

  .title-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-subtle);
  }

  .status-dot.active {
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
  }

  h3 { margin: 0; color: var(--text-strong); font-size: 14px; }
  p { margin: 4px 0 0; color: var(--muted); font-size: 12px; line-height: 1.35; }
  .update-status { color: var(--accent); }

  @media (max-width: 720px) {
    .update-panel { align-items: stretch; flex-direction: column; }
  }
</style>
