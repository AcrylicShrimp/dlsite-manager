<script lang="ts">
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
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<button
  class="button"
  class:secondary={variant === "secondary"}
  class:danger={variant === "danger"}
  class:small={size === "small"}
  class:auto-width={responsiveWidth === "auto"}
  {type}
  {disabled}
  {title}
  aria-label={ariaLabel}
  aria-expanded={ariaExpanded}
  {onclick}
>
  {@render children?.()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 84px;
    height: 38px;
    padding: 0 13px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    color: #09110c;
    background: var(--accent);
    font: inherit;
    letter-spacing: 0;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      background-color 120ms ease,
      color 120ms ease,
      transform 120ms ease;
  }

  .button.secondary {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--panel-raised);
  }

  .button.danger {
    border-color: var(--danger);
    color: var(--danger);
    background: rgb(248 113 113 / 10%);
  }

  .button.small {
    min-width: 62px;
    height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }

  .button:hover:not(:disabled) {
    border-color: #b3d8b8;
    background: #a8cfad;
  }

  .button.secondary:hover:not(:disabled) {
    border-color: var(--accent-strong);
    color: var(--text-strong);
    background: color-mix(in srgb, var(--panel-raised) 84%, var(--accent));
  }

  .button.danger:hover:not(:disabled) {
    border-color: #fca5a5;
    color: #fca5a5;
    background: rgb(248 113 113 / 16%);
  }

  .button:active:not(:disabled) {
    transform: translateY(1px);
  }

  .button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  @media (max-width: 720px) {
    .button {
      width: 100%;
    }

    .button.auto-width {
      width: auto;
    }
  }
</style>
