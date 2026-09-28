<script lang="ts">
  import Icon from "./DraftIcon.svelte";
  let {
    name,
    oncopy,
    onremove,
  }: {
    name: string;
    oncopy: () => void;
    onremove: () => void;
  } = $props();
</script>

<span
  class="tag-chip dm:inline-flex dm:items-stretch dm:max-w-full dm:rounded-md dm:border dm:border-solid dm:border-draft-tag-border dm:bg-draft-tag-bg dm:text-draft-tag-text"
>
  <button
    class="tag-label dm:inline-flex dm:min-w-0 dm:min-h-8 dm:items-center dm:px-2.5 dm:py-1 dm:border-0 dm:rounded-l-md dm:bg-transparent dm:text-inherit dm:font-[inherit] dm:text-xs dm:leading-normal dm:text-left dm:wrap-anywhere dm:cursor-pointer dm:hover:bg-draft-tag-hover dm:draft-focus-control"
    type="button"
    title={`Copy tag ${name}`}
    onclick={oncopy}>{name}</button
  >
  <button
    class="tag-remove dm:flex dm:w-8 dm:min-h-8 dm:shrink-0 dm:items-center dm:justify-center dm:p-1.5 dm:border-0 dm:rounded-r-md dm:bg-transparent dm:text-draft-tag-text dm:cursor-pointer dm:hover:bg-draft-tag-hover dm:draft-focus-control dm:[&_svg]:size-3"
    type="button"
    aria-label={`Remove custom tag ${name}`}
    title={`Remove ${name}`}
    onclick={onremove}><Icon name="close" /></button
  >
</span>
