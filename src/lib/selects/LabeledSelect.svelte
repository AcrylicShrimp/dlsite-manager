<script lang="ts">
  export let label: string;
</script>

<label class="flex flex-row items-center">
  <p>{label}:</p>
  <span class="flex-none inline-block w-2" />
  <select class="text-0/5 bg-4/5 rounded" on:change>
    <slot />
  </select>
</label>
