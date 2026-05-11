<script lang="ts">
  import type { ConfirmationDialog } from "$lib/model/types";

  let {
    dialog,
    onClose,
  }: {
    dialog: ConfirmationDialog | null;
    onClose: (confirmed: boolean) => void;
  } = $props();
</script>

{#if dialog}
  <div
    class="confirmation-dialog-layer"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirmation-dialog-title"
    aria-describedby="confirmation-dialog-message"
  >
    <button
      class="confirmation-dialog-backdrop"
      type="button"
      aria-label="Close confirmation dialog"
      onclick={() => onClose(false)}
    ></button>
    <section class:danger={dialog.tone === "danger"} class="confirmation-dialog-panel">
      <div class="confirmation-dialog-heading">
        <div>
          <p>{dialog.eyebrow}</p>
          <h2 id="confirmation-dialog-title">{dialog.title}</h2>
        </div>
        <button
          class="dialog-button confirmation-dialog-close"
          type="button"
          aria-label="Close confirmation dialog"
          onclick={() => onClose(false)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <p id="confirmation-dialog-message" class="confirmation-dialog-message">
        {dialog.message}
      </p>

      <div class="confirmation-dialog-actions">
        <button class="dialog-button secondary" type="button" onclick={() => onClose(false)}>
          {dialog.cancelLabel}
        </button>
        <button
          class="dialog-button"
          class:danger-action={dialog.tone === "danger"}
          type="button"
          onclick={() => onClose(true)}
        >
          {dialog.confirmLabel}
        </button>
      </div>
    </section>
  </div>
{/if}

<style>
  .confirmation-dialog-layer {
    position: fixed;
    z-index: 60;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .confirmation-dialog-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 68%);
    cursor: default;
  }

  .confirmation-dialog-panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(520px, calc(100vw - 40px));
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
  }

  .confirmation-dialog-panel.danger {
    border-color: rgb(248 113 113 / 42%);
  }

  .confirmation-dialog-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .confirmation-dialog-heading p,
  .confirmation-dialog-message {
    margin: 0;
  }

  .confirmation-dialog-heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .confirmation-dialog-heading h2 {
    margin: 2px 0 0;
    color: var(--text-strong);
    font-size: 20px;
    line-height: 1.2;
  }

  .dialog-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 84px;
    height: 38px;
    padding: 0 13px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    color: #09110c;
    background: var(--accent);
    cursor: pointer;
  }

  .dialog-button.secondary {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--panel-raised);
  }

  .confirmation-dialog-close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .confirmation-dialog-close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .confirmation-dialog-close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .confirmation-dialog-message {
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
  }

  .confirmation-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .danger-action {
    border-color: #fca5a5;
    color: #1b0707;
    background: #fca5a5;
  }

  .danger-action:hover:not(:disabled) {
    border-color: #fecaca;
    background: #fecaca;
  }
</style>
