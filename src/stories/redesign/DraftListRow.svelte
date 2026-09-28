<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./DraftIcon.svelte";
  let {
    children,
    leading,
    trailing,
    onclick,
    class: className = "",
  }: {
    children: Snippet;
    leading?: Snippet;
    trailing?: Snippet;
    onclick: () => void;
    class?: string;
  } = $props();
</script>

<button
  type="button"
  {onclick}
  class={`dm:box-border dm:flex dm:w-full dm:min-w-0 dm:items-center dm:gap-3 dm:px-4 dm:py-4 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:rounded-none dm:bg-transparent dm:text-left dm:text-draft-ink dm:font-[inherit] dm:leading-normal dm:cursor-pointer dm:hover:bg-draft-row-hover dm:draft-focus-row ${className}`}
>
  {#if leading}<span class="dm:shrink-0">{@render leading()}</span>{/if}
  <span
    class="dm:min-w-0 dm:flex-1 dm:flex dm:flex-col dm:gap-1 dm:wrap-anywhere"
    >{@render children()}</span
  >
  {#if trailing}<span
      class="dm:flex dm:shrink-0 dm:flex-col dm:items-end dm:gap-1 dm:text-xs dm:text-draft-dim"
      >{@render trailing()}</span
    >{/if}
  <span class="dm:flex dm:shrink-0 dm:text-draft-dim dm:max-[620px]:hidden"
    ><Icon name="arrow" /></span
  >
</button>
