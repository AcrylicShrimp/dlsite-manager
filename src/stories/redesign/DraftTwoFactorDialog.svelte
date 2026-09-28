<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { TwoFactorRequest } from "$lib/model/types";
  import Button from "./DraftButton.svelte";
  import Icon from "./DraftIcon.svelte";

  let {
    request,
    submitting = false,
    onSubmit,
    onCancel,
  }: {
    request: TwoFactorRequest;
    submitting?: boolean;
    onSubmit: (code: string) => void;
    onCancel: () => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let code = $state("");
  const canSubmit = $derived(Boolean(code.trim()) && !submitting);

  onMount(() => {
    dialog.showModal();
  });
  $effect(() => {
    const requestId = request.requestId;
    code = "";
    void tick().then(() => {
      if (dialog.open && request.requestId === requestId && !submitting)
        input.focus();
    });
  });
  function cancel() {
    if (submitting) return;
    dialog.close();
    onCancel();
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="draft-two-factor-title"
  aria-describedby="draft-two-factor-message"
  aria-busy={submitting}
  oncancel={(event) => {
    event.preventDefault();
    event.stopPropagation();
    cancel();
  }}
  onclose={(event) => event.stopPropagation()}
  class="dm:box-border dm:fixed dm:m-auto dm:w-[min(440px,92vw)] dm:max-w-none dm:max-h-[90dvh] dm:overflow-auto dm:rounded-xl dm:border dm:border-solid dm:border-draft-line-strong dm:bg-draft-surface dm:p-5 dm:text-draft-ink dm:backdrop:bg-draft-backdrop dm:backdrop:backdrop-blur-sm"
>
  <header class="dm:flex dm:items-start dm:justify-between dm:gap-3">
    <div class="dm:min-w-0">
      <h2
        id="draft-two-factor-title"
        class="dm:m-0 dm:text-lg dm:font-semibold"
      >
        Two-factor authentication
      </h2>
      <p class="dm:mb-0 dm:mt-1 dm:wrap-anywhere dm:text-sm dm:text-draft-dim">
        {request.accountLabel}
      </p>
    </div>
    <Button
      variant="icon"
      aria-label="Cancel two-factor verification"
      disabled={submitting}
      onclick={cancel}><Icon name="close" /></Button
    >
  </header>
  <p
    id="draft-two-factor-message"
    role={request.previousCodeRejected ? "alert" : undefined}
    class={`dm:my-5 dm:text-sm dm:leading-relaxed ${request.previousCodeRejected ? "dm:text-draft-error" : "dm:text-draft-dim"}`}
  >
    {request.previousCodeRejected
      ? "That code was rejected. Enter a new code from your authenticator app."
      : "Enter the code from your authenticator app."}
  </p>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      if (canSubmit) onSubmit(code.trim());
    }}
    class="dm:flex dm:flex-col dm:gap-4"
  >
    <label class="dm:flex dm:flex-col dm:gap-2 dm:text-sm">
      Verification code
      <input
        bind:this={input}
        bind:value={code}
        autocomplete="one-time-code"
        inputmode="numeric"
        maxlength={16}
        placeholder="123456"
        disabled={submitting}
        class="dm:box-border dm:w-full dm:min-w-0 dm:rounded-md dm:border dm:border-solid dm:border-draft-line-strong dm:bg-draft-input dm:px-3 dm:py-2.5 dm:text-base dm:text-draft-ink dm:font-[inherit] dm:draft-focus-field dm:disabled:opacity-40"
      />
    </label>
    {#if request.attempt > 1}<p class="dm:m-0 dm:text-xs dm:text-draft-dim">
        Attempt {request.attempt}
      </p>{/if}
    <div class="dm:flex dm:flex-wrap dm:justify-end dm:gap-2">
      <Button disabled={submitting} onclick={cancel}>Cancel</Button>
      <Button variant="primary" type="submit" disabled={!canSubmit}
        >{submitting ? "Verifying…" : "Verify"}</Button
      >
    </div>
  </form>
</dialog>
