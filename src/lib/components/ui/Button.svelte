<script lang="ts">
  import type { Snippet } from "svelte";

  type ButtonType = "button" | "submit" | "reset";
  type ButtonVariant = "primary" | "secondary" | "danger";
  type ButtonSize = "normal" | "small";

  let {
    children,
    type = "button",
    variant = "primary",
    size = "normal",
    disabled = false,
    title,
    ariaLabel,
    onclick,
  }: {
    children?: Snippet;
    type?: ButtonType;
    variant?: ButtonVariant;
    size?: ButtonSize;
    disabled?: boolean;
    title?: string;
    ariaLabel?: string;
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<button
  class="button"
  class:secondary={variant === "secondary"}
  class:danger={variant === "danger"}
  class:small={size === "small"}
  {type}
  {disabled}
  {title}
  aria-label={ariaLabel}
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

  .button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  @media (max-width: 720px) {
    .button {
      width: 100%;
    }
  }
</style>
