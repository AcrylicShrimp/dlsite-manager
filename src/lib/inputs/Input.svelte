<script lang="ts">
  export let placeholder: string;
  export let value: string;
</script>

<input
  type="text"
  {placeholder}
  bind:value
  on:change
  on:input
  class="px-2 py-1 w-full text-0/5 disabled:text-3/5 bg-4/5 disabled:bg-4/5/20 rounded"
  {...$$restProps}
/>
