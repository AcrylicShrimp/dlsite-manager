<script lang="ts">
  export let label: string;
  export let placeholder: string;
  export let value: string;
</script>

<label>
  <p>{label}</p>
  <div class="pl-2 pt-1">
    <input
      type="text"
      {placeholder}
      bind:value
      class="px-2 py-1 w-full text-0/5 disabled:text-3/5 bg-4/5 disabled:bg-4/5/20 rounded"
      {...$$restProps}
    />
  </div>
</label>
