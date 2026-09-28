<script lang="ts">
  import { onDestroy } from "svelte";
  import type { TwoFactorRequest } from "$lib/model/types";
  import CurrentDialog from "$lib/components/TwoFactorDialog.svelte";
  import DraftDialog from "./DraftTwoFactorDialog.svelte";
  import Button from "./DraftButton.svelte";
  let {
    variant,
    previewState = "mfa",
  }: {
    variant: "current" | "draft";
    previewState?: "mfa" | "mfa-rejected" | "mfa-submitting";
  } = $props();
  let request = $state<TwoFactorRequest | null>(null);
  let submitting = $state(false);
  let result = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  let serial = 0;
  function start() {
    clearTimeout(timer);
    result = "";
    submitting = previewState === "mfa-submitting";
    request = {
      requestId: `preview-mfa-${++serial}`,
      accountId: "secondary",
      accountLabel: "Secondary DLsite account",
      jobId: "preview-sync-secondary",
      attempt: previewState === "mfa-rejected" ? 2 : 1,
      previousCodeRejected: previewState === "mfa-rejected",
    };
  }
  $effect(() => {
    previewState;
    start();
  });
  onDestroy(() => clearTimeout(timer));
  function submit(code: string) {
    if (!request || submitting) return;
    const active = request;
    const accepted = code === "123456";
    submitting = true;
    timer = setTimeout(() => {
      if (request?.requestId !== active.requestId) return;
      submitting = false;
      if (accepted) {
        request = null;
        result = "Verification complete. Sync would resume.";
      } else {
        request = {
          ...active,
          requestId: `preview-mfa-${++serial}`,
          attempt: active.attempt + 1,
          previousCodeRejected: true,
        };
      }
    }, 650);
  }
  function cancel() {
    if (submitting) return;
    clearTimeout(timer);
    request = null;
    result = "Verification cancelled.";
  }
</script>

{#if request}
  {#if variant === "draft"}<DraftDialog
      {request}
      {submitting}
      onSubmit={submit}
      onCancel={cancel}
    />
  {:else}<CurrentDialog
      {request}
      {submitting}
      onSubmit={submit}
      onCancel={cancel}
    />{/if}
{:else if result}
  <div
    class="dm:fixed dm:bottom-4 dm:left-4 dm:right-4 dm:z-50 dm:flex dm:flex-wrap dm:items-center dm:justify-between dm:gap-3 dm:rounded-lg dm:border dm:border-solid dm:border-draft-line dm:bg-draft-surface dm:p-4 dm:text-sm dm:text-draft-ink"
  >
    <span role="status">{result}</span><Button onclick={start}>Try again</Button
    >
  </div>
{/if}
