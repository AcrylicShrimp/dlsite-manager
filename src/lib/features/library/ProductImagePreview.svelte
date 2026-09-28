<script lang="ts">
  import type { ProductImagePreview } from "$lib/model/types";
  import Modal from "$lib/components/workspace/Modal.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  let {
    preview,
    onClose,
    onSave,
  }: {
    preview: ProductImagePreview;
    onClose?: () => void;
    onSave: (workId: string) => Promise<boolean>;
  } = $props();
  let saving = $state(false);
  let message = $state("");
  let failed = $state(false);
  async function save() {
    if (saving) return;
    saving = true;
    message = "";
    failed = false;
    try {
      if (await onSave(preview.workId)) message = "Image saved.";
    } catch (error) {
      failed = true;
      message = String(error);
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  title={preview.title}
  wide
  dismissible={!saving}
  onClose={() => onClose?.()}
>
  <img
    src={preview.url}
    alt={`Cover of ${preview.title}`}
    class="dm:block dm:mx-auto dm:max-w-full dm:max-h-[65dvh] dm:object-contain"
  />
  {#if message}<p
      role={failed ? "alert" : "status"}
      class={failed ? "dm:text-draft-error" : "dm:text-draft-dim"}
    >
      {message}
    </p>{/if}
  {#snippet actions()}<Button disabled={saving} onclick={save}
      ><Icon name="downloads" />{saving ? "Saving…" : "Save image"}</Button
    >{/snippet}
</Modal>
