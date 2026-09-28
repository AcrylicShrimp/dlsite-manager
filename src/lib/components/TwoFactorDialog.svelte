<script lang="ts">
  import Modal from "$lib/components/workspace/Modal.svelte";
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
  <Modal
    title={`Verify ${request.accountLabel}`}
    onClose={onCancel}
    dismissible={!submitting}
  >
    <p
      id="two-factor-message"
      class="message"
      class:rejected={request.previousCodeRejected}
    >
      {#if request.previousCodeRejected}
        DLsite rejected that code. Open your authenticator app and enter the
        current code.
      {:else}
        DLsite asked for a verification code. Open your authenticator app and
        enter the current code for this account.
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
        <UiButton variant="secondary" disabled={submitting} onclick={onCancel}
          >Cancel</UiButton
        >
        <UiButton type="submit" disabled={!canSubmit}>
          {submitting ? "Verifying…" : "Verify"}
        </UiButton>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .message {
    margin: 0 0 20px;
    line-height: 1.5;
    color: var(--muted);
  }
  .rejected {
    color: var(--danger);
  }
  form {
    display: grid;
    gap: 20px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
