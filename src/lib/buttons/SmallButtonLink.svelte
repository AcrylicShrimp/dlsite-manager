<script lang="ts">
  export let href: string;
  export let target: string = "_blank";
</script>

<a
  {href}
  {target}
  class="duration-200 px-2 py-1 text-sm text-4/5 disabled:text-4/5/50 bg-3/5/50 hover:bg-3/5 disabled:bg-3/5/20 rounded"
  {...$$restProps}><slot /></a
>
