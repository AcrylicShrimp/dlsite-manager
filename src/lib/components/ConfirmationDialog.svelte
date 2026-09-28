<script lang="ts">
  import Modal from "$lib/components/workspace/Modal.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
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
  <Modal title={dialog.title} onClose={() => onClose(false)}>
    <p class="dm:m-0 dm:leading-relaxed">{dialog.message}</p>
    {#snippet actions()}
      <Button onclick={() => onClose(false)}>{dialog.cancelLabel}</Button>
      <Button
        variant={dialog.tone === "danger" ? "secondary" : "primary"}
        tone={dialog.tone === "danger" ? "error" : "normal"}
        onclick={() => onClose(true)}>{dialog.confirmLabel}</Button
      >
    {/snippet}
  </Modal>
{/if}
