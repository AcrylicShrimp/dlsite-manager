<script lang="ts">
  import { onMount } from "svelte";
  import Button from "./DraftButton.svelte";
  import Icon from "./DraftIcon.svelte";

  let {
    src,
    title,
    workId,
    onSave,
    onClose,
  }: {
    src: string;
    title: string;
    workId: string;
    onSave: () => string;
    onClose: () => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let saveMessage = $state("");

  function close() {
    dialog.close();
    onClose();
  }

  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  aria-label={`Cover image: ${title}`}
  onclose={(event) => event.stopPropagation()}
  oncancel={(event) => {
    event.preventDefault();
    event.stopPropagation();
    close();
  }}
  class="dm:box-border dm:fixed dm:m-auto dm:w-[min(920px,92vw)] dm:max-w-none dm:max-h-[90dvh] dm:overflow-auto dm:rounded-xl dm:border dm:border-solid dm:border-draft-line-strong dm:bg-draft-surface dm:p-0 dm:text-draft-ink dm:backdrop:bg-draft-backdrop dm:backdrop:backdrop-blur-sm"
>
  <header
    class="dm:flex dm:items-center dm:gap-3 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:px-4 dm:py-3"
  >
    <div class="dm:min-w-0 dm:flex-1">
      <h2 class="dm:m-0 dm:truncate dm:text-base dm:font-semibold">{title}</h2>
      <p class="dm:mt-1 dm:mb-0 dm:text-xs dm:text-draft-dim">{workId}</p>
    </div>
    <Button
      variant="secondary"
      onclick={() => {
        saveMessage = onSave();
      }}><Icon name="downloads" />Save image</Button
    >
    <Button variant="icon" aria-label="Close image preview" onclick={close}
      ><Icon name="close" /></Button
    >
  </header>
  <div
    class="dm:flex dm:items-center dm:justify-center dm:bg-draft-background dm:p-3"
  >
    <img
      class="dm:block dm:max-h-[65dvh] dm:max-w-full dm:object-contain"
      {src}
      alt={`${title} cover`}
    />
  </div>
  {#if saveMessage}<p
      role="status"
      class="dm:m-0 dm:px-4 dm:py-3 dm:text-sm dm:text-draft-dim"
    >
      {saveMessage}
    </p>{/if}
</dialog>
