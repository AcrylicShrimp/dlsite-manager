<script lang="ts">
  import type { TwoFactorRequest } from "$lib/model/types";

  let {
    request,
    submitting = false,
    onSubmit,
    onCancel,
  }: {
    request: TwoFactorRequest | null;
    submitting?: boolean;
    onSubmit: (code: string) => void;
    onCancel: () => void;
  } = $props();

  let code = $state("");

  // Clearing on each new request keeps a rejected code from being resubmitted, and keeps one
  // account's code from leaking into another account's prompt.
  $effect(() => {
    request?.requestId;
    code = "";
  });

  const trimmedCode = $derived(code.trim());
  const canSubmit = $derived(trimmedCode.length > 0 && !submitting);

  function submit(event: SubmitEvent) {
    event.preventDefault();

    if (canSubmit) {
      onSubmit(trimmedCode);
    }
  }
</script>

{#if request}
  <div
    class="two-factor-layer"
    role="dialog"
    aria-modal="true"
    aria-labelledby="two-factor-title"
    aria-describedby="two-factor-message"
  >
    <button
      class="two-factor-backdrop"
      type="button"
      aria-label="Cancel two-factor verification"
      onclick={onCancel}
    ></button>
    <section class="two-factor-panel">
      <div class="two-factor-heading">
        <div>
          <p>Two-factor authentication</p>
          <h2 id="two-factor-title">{request.accountLabel}</h2>
        </div>
        <button
          class="dialog-button two-factor-close"
          type="button"
          aria-label="Cancel two-factor verification"
          onclick={onCancel}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <p id="two-factor-message" class="two-factor-message">
        {#if request.previousCodeRejected}
          DLsite rejected that code. Open your authenticator app and enter the current code.
        {:else}
          DLsite asked for a verification code. Open your authenticator app and enter the
          current code for this account.
        {/if}
      </p>

      <form class="two-factor-form" onsubmit={submit}>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="two-factor-input"
          type="text"
          inputmode="numeric"
          autocomplete="one-time-code"
          autocapitalize="off"
          autocorrect="off"
          spellcheck="false"
          maxlength="16"
          placeholder="123456"
          aria-label="Verification code"
          autofocus
          disabled={submitting}
          bind:value={code}
        />

        <div class="two-factor-actions">
          <button
            class="dialog-button secondary"
            type="button"
            disabled={submitting}
            onclick={onCancel}
          >
            Cancel
          </button>
          <button class="dialog-button" type="submit" disabled={!canSubmit}>
            {submitting ? "Verifying…" : "Verify"}
          </button>
        </div>
      </form>
    </section>
  </div>
{/if}

<style>
  .two-factor-layer {
    position: fixed;
    z-index: 70;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .two-factor-backdrop {
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

  .two-factor-panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(440px, calc(100vw - 40px));
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
  }

  .two-factor-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .two-factor-heading p,
  .two-factor-message {
    margin: 0;
  }

  .two-factor-heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .two-factor-heading h2 {
    margin: 2px 0 0;
    overflow: hidden;
    color: var(--text-strong);
    font-size: 20px;
    line-height: 1.2;
    text-overflow: ellipsis;
  }

  .two-factor-message {
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
  }

  .two-factor-form {
    display: grid;
    gap: 16px;
  }

  .two-factor-input {
    width: 100%;
    height: 44px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text-strong);
    font-size: 20px;
    font-family: inherit;
    letter-spacing: 0.28em;
    background: var(--panel-raised);
  }

  .two-factor-input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .two-factor-input:disabled {
    opacity: 0.6;
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

  .dialog-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dialog-button.secondary {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--panel-raised);
  }

  .two-factor-close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .two-factor-close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .two-factor-close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .two-factor-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
