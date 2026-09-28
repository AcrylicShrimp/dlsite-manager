<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./DraftIcon.svelte";
  let {
    title,
    children,
    nested = false,
  }: { title: string; children: Snippet; nested?: boolean } = $props();
</script>

<details
  class={`dm:[&[open]>summary>span:first-child]:rotate-90 ${nested ? "dm:mt-3" : "metadata-section dm:border-0 dm:border-b dm:border-solid dm:border-draft-line"}`}
>
  <summary
    class="dm:flex dm:min-h-11 dm:cursor-pointer dm:list-none dm:items-center dm:gap-2 dm:rounded-md dm:px-3 dm:py-3 dm:text-sm dm:text-draft-ink dm:draft-focus-row dm:hover:bg-draft-row-hover dm:[&::-webkit-details-marker]:hidden"
  >
    <span class="dm:flex dm:shrink-0"><Icon name="arrow" /></span><span
      >{title}</span
    >
  </summary>
  <div class="dm:px-3 dm:pb-4 dm:pt-2">{@render children()}</div>
</details>
