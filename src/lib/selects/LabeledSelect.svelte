<script lang="ts">
  export let label: string;
  export let value: string;
</script>

<label class="flex flex-row items-center">
  <p class="min-w-[5em]">{label}:</p>
  <span class="flex-none inline-block w-2" />
  <select class="text-0/5 bg-4/5 rounded" bind:value on:change>
    <slot />
  </select>
</label>
