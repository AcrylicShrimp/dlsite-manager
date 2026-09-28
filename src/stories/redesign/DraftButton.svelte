<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  let {
    children,
    variant = "secondary",
    tone = "normal",
    class: className = "",
    type = "button",
    ...attributes
  }: HTMLButtonAttributes & {
    children?: Snippet;
    variant?:
      | "primary"
      | "secondary"
      | "text"
      | "sidebar"
      | "navigation"
      | "icon"
      | "field-icon";
    tone?: "normal" | "muted" | "error";
  } = $props();
  const sidebar =
    "dm:w-full dm:min-h-10 dm:px-2.5 dm:py-2 dm:justify-start dm:border-0 dm:bg-transparent dm:focus-visible:bg-draft-focus-bg dm:aria-[current=page]:bg-draft-selected dm:aria-[current=page]:text-draft-ink dm:enabled:hover:bg-draft-hover";
  const variants = {
    primary:
      "dm:min-h-9 dm:px-3 dm:py-2 dm:justify-center dm:border-0 dm:bg-draft-accent dm:text-draft-on-accent dm:[--dm-shadow-draft-focus-control:var(--dm-shadow-draft-focus-on-accent)] dm:font-semibold dm:enabled:hover:bg-draft-accent-hover",
    secondary:
      "dm:min-h-9 dm:px-3 dm:py-2 dm:justify-center dm:border dm:border-solid dm:border-draft-line dm:bg-draft-elevated dm:enabled:hover:bg-draft-hover",
    text: "dm:min-h-8 dm:px-2.5 dm:py-1.5 dm:justify-center dm:border-0 dm:bg-transparent dm:enabled:hover:bg-draft-hover",
    icon: "dm:size-8 dm:p-1.5 dm:justify-center dm:shrink-0 dm:border-0 dm:bg-transparent dm:enabled:hover:bg-draft-hover",
    "field-icon":
      "dm:size-8 dm:p-1.5 dm:justify-center dm:shrink-0 dm:border-0 dm:bg-transparent dm:enabled:hover:text-draft-accent",
    sidebar,
    navigation: `${sidebar} dm:max-[620px]:flex-1 dm:max-[620px]:flex-col dm:max-[620px]:gap-1 dm:max-[620px]:px-0.5 dm:max-[620px]:text-[9px]`,
  };
</script>

<button
  {type}
  {...attributes}
  data-variant={variant}
  class={`draft-button dm:box-border dm:inline-flex dm:min-w-0 dm:items-center dm:gap-2 dm:rounded-md dm:text-left dm:font-[inherit] dm:text-sm dm:leading-normal dm:cursor-pointer dm:draft-focus-control dm:disabled:opacity-40 dm:disabled:cursor-default ${variants[variant]} ${variant === "primary" ? "" : tone === "error" ? "dm:text-draft-error" : tone === "muted" || variant === "sidebar" || variant === "navigation" ? "dm:text-draft-dim" : variant === "text" ? "dm:text-draft-accent" : "dm:text-draft-ink"} ${className}`}
>
  {@render children?.()}
</button>
