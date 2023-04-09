<script lang="ts">
  let menu = false;

  function onClick() {
    menu = !menu;
  }

  function onMouseLeave() {
    menu = false;
  }
</script>

<div
  class="relative flex items-stretch justify-start text-sm text-4/5 disabled:text-4/5/50 disabled:bg-error/20"
>
  <button
    class="duration-200 py-1 w-[8.5rem] bg-error/50 hover:bg-error rounded-l"
    on:click><slot /></button
  >
  <button
    class="duration-200 py-1 w-6 bg-error/30 hover:bg-error rounded-r"
    on:click={onClick}><slot name="right" /></button
  >
  {#if menu}
    <div
      class="absolute top-[calc(100%+0.25rem)] left-2 right-2 border border-2/5 bg-1/5 z-10 rounded"
      on:mouseleave={onMouseLeave}
    >
      <slot name="menu" />
    </div>
  {/if}
</div>
