<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    eyebrow,
    title,
    actions,
  }: {
    eyebrow: string;
    title: string;
    actions?: Snippet;
  } = $props();
</script>

<header class="workspace-header">
  <div class="workspace-heading">
    <p>{eyebrow}</p>
    <h1>{title}</h1>
  </div>
  {#if actions}
    <div class="workspace-header-actions">
      {@render actions()}
    </div>
  {/if}
</header>

<style>
  .workspace-header {
    display: flex;
    flex: 0 0 auto;
    justify-content: space-between;
    gap: 16px;
    align-items: center;
    margin-bottom: 14px;
  }

  .workspace-heading {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  p,
  h1 {
    margin: 0;
  }

  p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.055em;
    text-transform: uppercase;
  }

  h1 {
    color: var(--text-strong);
    font-size: 28px;
    font-weight: 720;
    line-height: 1.12;
  }

  .workspace-header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  @media (max-width: 720px) {
    .workspace-header {
      align-items: stretch;
      flex-direction: column;
    }

    h1 {
      font-size: 25px;
    }
  }
</style>
