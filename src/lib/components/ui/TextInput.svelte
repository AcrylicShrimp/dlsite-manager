<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  let {
    value = $bindable(""),
    id,
    type = "text",
    autocomplete = "off",
    spellcheck = false,
    disabled = false,
    placeholder,
    ariaLabel,
  }: {
    value?: string;
    id?: string;
    type?: "text" | "search" | "password";
    autocomplete?: HTMLInputAttributes["autocomplete"];
    spellcheck?: boolean;
    disabled?: boolean;
    placeholder?: string;
    ariaLabel?: string;
  } = $props();
</script>

<input
  {id}
  {type}
  {autocomplete}
  {spellcheck}
  {disabled}
  {placeholder}
  aria-label={ariaLabel}
  bind:value
/>

<style>
  input {
    width: 100%;
    min-width: 0;
    height: 38px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    background: var(--field);
    font: inherit;
    letter-spacing: 0;
  }

  input:focus {
    border-color: var(--accent-strong);
    outline: 2px solid var(--accent-muted);
  }

  input:disabled {
    color: var(--text-subtle);
    background: var(--field-disabled);
  }
</style>
