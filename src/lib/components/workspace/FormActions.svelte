<script lang="ts">
  import type { Snippet } from "svelte";
  let { children }: { children: Snippet } = $props();
</script>

<div
  class="form-actions dm:flex dm:flex-wrap dm:items-center dm:justify-end dm:gap-2 dm:border-0 dm:border-t dm:border-solid dm:border-draft-line dm:pt-4"
>
  {@render children()}
</div>
