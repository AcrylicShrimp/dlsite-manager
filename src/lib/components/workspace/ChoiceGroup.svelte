<script lang="ts">
  let {
    label,
    options,
    value,
    onchange,
    variant = "filter",
  }: {
    label: string;
    options: { value: string; label: string; count?: number }[];
    value: string;
    onchange: (value: string) => void;
    variant?: "filter" | "tabs";
  } = $props();
</script>

<div
  role="group"
  aria-label={label}
  class={variant === "tabs"
    ? "tabs dm:flex dm:flex-wrap dm:gap-1 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line"
    : "switches dm:flex dm:flex-wrap dm:gap-1"}
>
  {#each options as option}
    <button
      type="button"
      aria-pressed={value === option.value}
      onclick={() => onchange(option.value)}
      class={`dm:box-border dm:inline-flex dm:items-center dm:justify-center dm:gap-2 dm:min-h-8 dm:px-3 dm:py-2 dm:font-[inherit] dm:text-sm dm:leading-normal dm:cursor-pointer dm:draft-focus-control dm:hover:bg-draft-hover ${variant === "tabs" ? `dm:min-h-10 dm:border-0 dm:border-b-2 dm:border-solid dm:rounded-t-md dm:bg-transparent ${value === option.value ? "dm:border-draft-accent dm:text-draft-ink" : "dm:border-transparent dm:text-draft-dim"}` : `dm:rounded-md dm:border-0 ${value === option.value ? "dm:bg-draft-selected dm:text-draft-ink" : "dm:bg-transparent dm:text-draft-dim"}`} `}
    >
      {option.label}{#if option.count !== undefined}<span
          class="dm:text-xs dm:text-draft-dim">{option.count}</span
        >{/if}
    </button>
  {/each}
</div>
