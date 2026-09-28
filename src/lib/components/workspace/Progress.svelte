<script lang="ts">
  let {
    value,
    label,
    compact = false,
  }: { value: number | null; label: string; compact?: boolean } = $props();
</script>

<progress
  max="100"
  value={value ?? undefined}
  aria-label={label}
  class={`dm:block dm:w-full dm:appearance-none dm:border-0 dm:rounded dm:overflow-hidden dm:bg-draft-progress-track dm:text-draft-accent ${compact ? "dm:h-0.5" : "dm:h-1"}`}
></progress>

<style>
  progress::-webkit-progress-bar {
    background: var(--dm-color-draft-progress-track);
  }
  progress::-webkit-progress-value {
    background: var(--dm-color-draft-accent);
  }
  progress::-moz-progress-bar {
    background: var(--dm-color-draft-accent);
  }
</style>
