<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, children }: { title: string; children?: Snippet } = $props();
</script>

<header
  class="dm:mb-6 dm:flex dm:flex-wrap dm:items-center dm:justify-between dm:gap-3"
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
