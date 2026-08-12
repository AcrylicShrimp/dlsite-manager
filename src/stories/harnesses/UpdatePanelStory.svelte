<script lang="ts">
  import UpdatePanel from "$lib/features/settings/UpdatePanel.svelte";

  let {
    phase = "idle",
    message = "",
  }: {
    phase?: "idle" | "checking" | "downloading" | "installing";
    message?: string;
  } = $props();

  let action = $state("No action yet");
</script>

<main class="story-surface">
  <div class="panel-shell">
    <UpdatePanel {phase} {message} onCheck={() => (action = "Check for updates")} />
  </div>
  <p aria-live="polite">{action}</p>
</main>

<style>
  .story-surface { display: grid; gap: 12px; width: min(720px, calc(100vw - 48px)); margin: 24px auto; }
  .panel-shell { padding: 18px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); }
  p { margin: 0; color: var(--text-subtle); font-size: 12px; text-align: right; }
  @media (max-width: 720px) { .story-surface { width: calc(100vw - 24px); margin: 12px auto; } }
</style>
