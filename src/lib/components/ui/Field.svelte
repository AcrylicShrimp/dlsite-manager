<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    id,
    label,
    help,
    children,
  }: {
    id?: string;
    label: string;
    help?: string;
    children?: Snippet;
  } = $props();
</script>

<div class="field">
  <label for={id}>
    <span>{label}</span>
    {#if help}
      <small>{help}</small>
    {/if}
  </label>
  {@render children?.()}
</div>

<style>
  .field {
    display: grid;
    gap: 8px;
  }

  label {
    display: grid;
    gap: 6px;
  }

  span {
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
  }

  small {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
  }
</style>
