<script lang="ts">
  import ProductDetailDialog from "$lib/features/library/ProductDetailDialog.svelte";
  import type { ProductCreditField, ProductDetail } from "$lib/model/types";

  let { detail }: { detail: ProductDetail } = $props();

  let customTagInput = $state("");
  let lastAction = $state("Detail open");

  function copyText(label: string, value: string | null, workId: string) {
    lastAction = `Copy ${label} for ${workId}: ${value ?? "-"}`;
  }

  function copyCredit(field: ProductCreditField, workId: string) {
    lastAction = `Copy ${field.label} for ${workId}`;
  }
</script>

<ProductDetailDialog
  {detail}
  bind:customTagInput
  onClose={() => (lastAction = "Close detail")}
  onPreview={() => (lastAction = `Preview ${detail.workId}`)}
  onCopyText={copyText}
  onCopyWorkId={() => (lastAction = `Copy ${detail.workId}`)}
  onCopyCredit={copyCredit}
  onOpenDlsite={() => (lastAction = `Open ${detail.workId} on DLsite`)}
  onAddTags={() => {
    lastAction = `Add tags: ${customTagInput}`;
    customTagInput = "";
  }}
  onRemoveTag={(name) => (lastAction = `Remove tag: ${name}`)}
/>

<p aria-live="polite">{lastAction}</p>

<style>
  p {
    position: fixed;
    z-index: 55;
    right: 14px;
    bottom: 10px;
    max-width: calc(100vw - 28px);
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
