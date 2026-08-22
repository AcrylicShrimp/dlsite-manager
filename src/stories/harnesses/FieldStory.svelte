<script lang="ts">
  import Field from "$lib/components/ui/Field.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";

  let {
    label = "Library folder",
    help = "Downloaded works are finalized under this folder.",
    placeholder = "/Users/example/Library",
    disabled = false,
    type = "text",
  }: {
    label?: string;
    help?: string;
    placeholder?: string;
    disabled?: boolean;
    type?: "text" | "search" | "password";
  } = $props();

  let value = $state("");
</script>

<div class="field-story">
  <Field id="storybook-field" {label} {help}>
    <TextInput id="storybook-field" bind:value {placeholder} {disabled} {type} />
  </Field>
</div>

<style>
  .field-story {
    width: min(420px, calc(100vw - 48px));
  }
</style>
