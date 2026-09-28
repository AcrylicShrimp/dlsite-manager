<script lang="ts">
  import { registerModal } from "./modal-stack";
  import { onDestroy, untrack } from "svelte";
  import type { Snippet } from "svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  let {
    title,
    children,
    actions,
    open = true,
    onClose,
    dismissible = true,
    wide = false,
  }: {
    title: string;
    children: Snippet;
    actions?: Snippet;
    open?: boolean;
    onClose: () => void;
    dismissible?: boolean;
    wide?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  let unregister: (() => void) | undefined;
  $effect(() => {
    const visible = open;
    untrack(() => {
      if (!dialog) return;
      if (visible && !dialog.open) {
        dialog.showModal();
        unregister = registerModal(dialog);
      } else if (!visible && dialog.open) {
        unregister?.();
        unregister = undefined;
        dialog.close();
      }
    });
  });
  onDestroy(() => {
    unregister?.();
    if (dialog?.open) dialog.close();
  });
</script>

<dialog
  bind:this={dialog}
  aria-label={title}
  onkeydown={(e) => {
    if (e.key === "Escape") e.stopPropagation();
  }}
  oncancel={(e) => {
    e.preventDefault();
    e.stopPropagation();
    if (dismissible) onClose();
  }}
  onclose={(e) => e.stopPropagation()}
  class={`workspace-modal dm:box-border dm:fixed dm:m-auto dm:max-w-none dm:max-h-[90dvh] dm:overflow-auto dm:rounded-xl dm:border dm:border-solid dm:border-draft-line-strong dm:bg-draft-surface dm:p-0 dm:text-draft-ink dm:backdrop:bg-draft-backdrop dm:backdrop:backdrop-blur-sm ${wide ? "dm:w-[min(760px,92vw)]" : "dm:w-[min(480px,92vw)]"}`}
>
  <header
    class="dm:sticky dm:top-0 dm:z-10 dm:flex dm:items-center dm:justify-between dm:gap-3 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:bg-draft-surface dm:px-6 dm:py-3.5"
  >
    <h2
      tabindex="-1"
      class="dm:outline-none dm:m-0 dm:min-w-0 dm:wrap-anywhere dm:text-base dm:font-semibold"
    >
      {title}
    </h2>
    <Button
      variant="icon"
      aria-label={`Close ${title}`}
      disabled={!dismissible}
      onclick={onClose}><Icon name="close" /></Button
    >
  </header>
  <div class="dm:px-6 dm:py-5">
    {@render children()}
    {#if actions}<footer
        class="dm:mt-5 dm:flex dm:flex-wrap dm:justify-end dm:gap-2"
      >
        {@render actions()}
      </footer>{/if}
  </div>
</dialog>
