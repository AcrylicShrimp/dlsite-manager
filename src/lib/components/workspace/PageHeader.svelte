<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    title,
    children,
    tabs,
    toolbar,
  }: {
    title: string;
    children?: Snippet;
    tabs?: Snippet;
    toolbar?: Snippet;
  } = $props();
</script>

<div class="page-header dm:mb-6 dm:flex dm:min-w-0 dm:flex-col dm:gap-4">
  <header
    class="dm:flex dm:min-h-10 dm:flex-wrap dm:items-center dm:justify-between dm:gap-3"
  >
    <h1
      tabindex="-1"
      class="dm:outline-none dm:m-0 dm:text-[28px] dm:font-bold dm:tracking-tight"
    >
      {title}
    </h1>
    {#if children}<div class="dm:flex dm:flex-wrap dm:items-center dm:gap-2">
        {@render children()}
      </div>{/if}
  </header>
  {@render tabs?.()}
  {@render toolbar?.()}
</div>
