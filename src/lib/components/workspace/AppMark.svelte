<script lang="ts">
  let { size = "small" }: { size?: "small" | "large" } = $props();
</script>

<span
  aria-hidden="true"
  class={`app-mark dm:box-border dm:grid dm:shrink-0 dm:place-items-center dm:rounded-lg dm:border dm:border-solid dm:border-draft-selected-border dm:bg-draft-selected dm:text-draft-accent dm:font-semibold ${size === "large" ? "dm:size-12 dm:text-xl" : "dm:size-8 dm:text-base dm:max-[620px]:size-6"}`}
  >dm</span
>
