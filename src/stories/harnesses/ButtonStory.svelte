<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";

  let {
    label = "Continue",
    variant = "primary",
    size = "normal",
    disabled = false,
  }: {
    label?: string;
    variant?: "primary" | "secondary" | "danger";
    size?: "normal" | "small";
    disabled?: boolean;
  } = $props();
</script>

<UiButton {variant} {size} {disabled}>{label}</UiButton>
