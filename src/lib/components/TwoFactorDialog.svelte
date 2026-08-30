<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import Field from "$lib/components/ui/Field.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";
  import type { TwoFactorRequest } from "$lib/model/types";

  const CODE_FIELD_ID = "two-factor-code";

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
  // account's code from leaking into another account's prompt. The dialog opens on the job's
  // schedule rather than a user gesture, so focus is moved into it as well.
  $effect(() => {
    if (!request?.requestId) {
      return;
    }

    code = "";
    document.getElementById(CODE_FIELD_ID)?.focus();
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
    class="dialog-layer"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="two-factor-title"
    aria-describedby="two-factor-message"
    onkeydown={(event) => {
      if (event.key === "Escape") onCancel();
    }}
  >
    <button
      class="backdrop"
      type="button"
      aria-label="Cancel two-factor verification"
      onclick={onCancel}
    ></button>
    <section class="panel">
      <div class="heading">
        <div>
          <p>Two-factor authentication</p>
          <h2 id="two-factor-title">{request.accountLabel}</h2>
        </div>
        <button
          class="close"
          type="button"
          aria-label="Cancel two-factor verification"
          onclick={onCancel}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <p id="two-factor-message" class="message" class:rejected={request.previousCodeRejected}>
        {#if request.previousCodeRejected}
          DLsite rejected that code. Open your authenticator app and enter the current code.
        {:else}
          DLsite asked for a verification code. Open your authenticator app and enter the
          current code for this account.
        {/if}
      </p>

      <form onsubmit={submit}>
        <Field
          id={CODE_FIELD_ID}
          label="Verification code"
          help={request.attempt > 1 ? `Attempt ${request.attempt}` : undefined}
        >
          <TextInput
            id={CODE_FIELD_ID}
            autocomplete="one-time-code"
            inputmode="numeric"
            maxlength={16}
            placeholder="123456"
            disabled={submitting}
            bind:value={code}
          />
        </Field>

        <div class="actions">
          <UiButton variant="secondary" disabled={submitting} onclick={onCancel}>Cancel</UiButton>
          <UiButton type="submit" disabled={!canSubmit}>
            {submitting ? "Verifying…" : "Verify"}
          </UiButton>
        </div>
      </form>
    </section>
  </div>
{/if}

<style>
  .dialog-layer {
    position: fixed;
    z-index: 70;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .backdrop {
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

  .panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(440px, calc(100vw - 40px));
    max-height: calc(100vh - 48px);
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
    overflow: auto;
  }

  .heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .heading p,
  .message {
    margin: 0;
  }

  .heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  h2 {
    margin: 2px 0 0;
    min-width: 0;
    color: var(--text-strong);
    font-size: 20px;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .close:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .message {
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
  }

  .message.rejected {
    padding: 10px 12px;
    border: 1px solid rgb(248 113 113 / 36%);
    border-radius: 8px;
    color: #fecaca;
    background: rgb(248 113 113 / 11%);
    font-size: 13px;
    line-height: 1.45;
  }

  form {
    display: grid;
    gap: 16px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @media (max-width: 720px) {
    .dialog-layer {
      padding: 12px;
    }

    .panel {
      width: 100%;
      max-height: calc(100vh - 24px);
      padding: 14px;
    }

    .actions {
      display: grid;
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
