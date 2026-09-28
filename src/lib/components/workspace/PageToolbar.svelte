<script lang="ts">
  import type { Snippet } from "svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";

  let {
    label,
    children,
    actions,
    onReload,
    reloadDisabled = false,
  }: {
    label: string;
    children?: Snippet;
    actions?: Snippet;
    onReload?: () => void;
    reloadDisabled?: boolean;
  } = $props();
</script>

<div
  role="group"
  aria-label={label}
  class="page-toolbar dm:flex dm:min-w-0 dm:flex-wrap dm:items-center dm:gap-3"
>
  {#if children}
    <div
      class="page-toolbar-controls dm:flex dm:min-w-0 dm:basis-80 dm:grow dm:flex-wrap dm:items-center dm:gap-2"
    >
      {@render children()}
    </div>
  {/if}
  {#if actions || onReload}
    <div
      class="page-toolbar-actions dm:ml-auto dm:flex dm:min-w-0 dm:flex-wrap dm:items-center dm:justify-end dm:gap-2"
    >
      {@render actions?.()}
      {#if onReload}
        <Button disabled={reloadDisabled} onclick={onReload}
          ><Icon name="refresh" />Reload</Button
        >
      {/if}
    </div>
  {/if}
</div>
