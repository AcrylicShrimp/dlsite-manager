<script lang="ts">
  import type { Toast } from "$lib/model/types";

  let {
    toasts,
    onDismiss,
  }: {
    toasts: Toast[];
    onDismiss: (id: string) => void;
  } = $props();
</script>

{#if toasts.length > 0}
  <section class="toast-stack" aria-label="Notifications" aria-live="polite">
    {#each toasts as toast (toast.id)}
      <article
        class="toast"
        class:error={toast.kind === "error"}
        class:success={toast.kind === "success"}
        role={toast.kind === "error" ? "alert" : "status"}
      >
        <div class="toast-marker" aria-hidden="true"></div>
        <p>{toast.message}</p>
        <button
          class="toast-close"
          type="button"
          aria-label="Dismiss notification"
          onclick={() => onDismiss(toast.id)}
        >
          <svg class="toast-close-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </article>
    {/each}
  </section>
{/if}

<style>
  .toast-stack {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 120;
    display: grid;
    gap: 8px;
    width: min(380px, calc(100vw - 36px));
    pointer-events: none;
  }

  .toast {
    --toast-color: #8ab4e6;
    --toast-bg: rgb(138 180 230 / 12%);

    display: grid;
    grid-template-columns: 4px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    min-height: 48px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel-raised) 92%, black);
    box-shadow: 0 18px 42px rgb(0 0 0 / 38%);
    overflow: hidden;
    pointer-events: auto;
  }

  .toast.success {
    --toast-color: var(--accent);
    --toast-bg: var(--accent-muted);
  }

  .toast.error {
    --toast-color: var(--danger);
    --toast-bg: rgb(248 113 113 / 13%);
  }

  .toast-marker {
    align-self: stretch;
    background: var(--toast-color);
  }

  .toast p {
    min-width: 0;
    margin: 0;
    padding: 10px 0;
    color: var(--text);
    font-size: 13px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .toast-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    min-width: 30px;
    height: 30px;
    margin-right: 8px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--muted);
    background: transparent;
    cursor: pointer;
  }

  .toast-close:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--toast-bg);
  }

  .toast-close-icon {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.2;
  }
</style>
