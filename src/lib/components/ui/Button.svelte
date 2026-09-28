<script lang="ts">
  import Button from "$lib/components/workspace/Button.svelte";
  import type { Snippet } from "svelte";

  type ButtonType = "button" | "submit" | "reset";
  type ButtonVariant = "primary" | "secondary" | "danger";
  type ButtonSize = "normal" | "small";
  type ButtonResponsiveWidth = "fill" | "auto";

  let {
    children,
    type = "button",
    variant = "primary",
    size = "normal",
    responsiveWidth = "fill",
    disabled = false,
    title,
    ariaLabel,
    ariaExpanded,
    ariaControls,
    onclick,
  }: {
    children?: Snippet;
    type?: ButtonType;
    variant?: ButtonVariant;
    size?: ButtonSize;
    responsiveWidth?: ButtonResponsiveWidth;
    disabled?: boolean;
    title?: string;
    ariaLabel?: string;
    ariaExpanded?: boolean;
    ariaControls?: string;
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<Button
  {type}
  {disabled}
  {title}
  variant={variant === "danger" ? "secondary" : variant}
  tone={variant === "danger" ? "error" : "normal"}
  aria-label={ariaLabel}
  aria-expanded={ariaExpanded}
  aria-controls={ariaControls}
  {onclick}
  class={responsiveWidth === "fill" ? "dm:max-[620px]:w-full" : ""}
>
  {@render children?.()}
</Button>
