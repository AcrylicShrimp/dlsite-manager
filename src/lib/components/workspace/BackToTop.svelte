<script lang="ts">
  import Icon from "./Icon.svelte";
  let {
    target,
    class: className = "",
  }: { target?: () => HTMLElement | null; class?: string } = $props();
  function top(event: MouseEvent) {
    const scroller =
      target?.() ?? (event.currentTarget as HTMLElement).closest("dialog");
    if (!scroller) return;
    scroller
      .querySelector<HTMLElement>("h1, h2")
      ?.focus({ preventScroll: true });
    scroller.scrollTo({
      top: 0,
      behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
    });
  }
</script>

<button
  type="button"
  aria-label="Back to top"
  title="Back to top"
  onclick={top}
  class={`dm:grid dm:size-10 dm:shrink-0 dm:place-items-center dm:rounded-full dm:border dm:border-solid dm:border-draft-selected-border dm:bg-draft-selected-strong dm:p-0 dm:text-draft-ink dm:cursor-pointer dm:draft-focus-control ${className}`}
  ><Icon name="up" /></button
>
