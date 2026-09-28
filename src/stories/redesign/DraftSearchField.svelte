<script lang="ts">
  import Button from "./DraftButton.svelte";
  import Icon from "./DraftIcon.svelte";
  let {
    value = $bindable(""),
    label,
    placeholder,
    compact = false,
    clearable = false,
  }: {
    value: string;
    label: string;
    placeholder: string;
    compact?: boolean;
    clearable?: boolean;
  } = $props();
  let input: HTMLInputElement;
</script>

<div
  class="search-field dm:flex dm:min-w-0 dm:flex-1 dm:items-center dm:gap-1 dm:rounded-md dm:border dm:border-solid dm:border-draft-line-subtle dm:bg-draft-input dm:px-1 dm:text-draft-dim dm:focus-within:border-draft-focus"
>
  <span class="dm:flex dm:size-8 dm:shrink-0 dm:items-center dm:justify-center"
    ><Icon name="search" /></span
  >
  <input
    bind:this={input}
    bind:value
    aria-label={label}
    {placeholder}
    class={`dm:min-w-0 dm:w-full dm:border-0 dm:bg-transparent dm:py-0 dm:pl-0 ${clearable ? "dm:pr-0" : "dm:pr-2"} dm:font-[inherit] dm:text-draft-ink dm:outline-none dm:placeholder:text-draft-placeholder ${compact ? "dm:h-8 dm:text-xs" : "dm:h-9 dm:text-base"}`}
  />
  {#if clearable}<span
      class="search-clear-slot dm:flex dm:size-8 dm:shrink-0 dm:items-center dm:justify-center"
      >{#if value}<Button
          variant="field-icon"
          tone="muted"
          aria-label="Clear search"
          onclick={() => {
            value = "";
            input.focus();
          }}><Icon name="close" /></Button
        >{/if}</span
    >{/if}
</div>
