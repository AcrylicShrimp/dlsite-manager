<script lang="ts">
  import ProductCard from "$lib/features/library/ProductCard.svelte";
  import type { Product, ProductCreditField } from "$lib/model/types";

  let {
    product,
    downloadLabel = "Download",
    downloadTitle = "Download this work",
    downloadDisabled = false,
    detailLoading = false,
    menuOpen = false,
  }: {
    product: Product;
    downloadLabel?: string;
    downloadTitle?: string;
    downloadDisabled?: boolean;
    detailLoading?: boolean;
    menuOpen?: boolean;
  } = $props();

  let lastAction = $state("No action yet");

  function action(label: string) {
    lastAction = label;
  }

  function copyCredit(field: ProductCreditField, workId: string) {
    action(`Copy ${field.label} for ${workId}`);
  }
</script>

<main class="story-surface">
  <section class="card-frame">
    <ProductCard
      {product}
      {downloadLabel}
      {downloadTitle}
      {downloadDisabled}
      {detailLoading}
      {menuOpen}
      onPreview={() => action(`Preview ${product.workId}`)}
      onOpenDetails={() => action(`Open details for ${product.workId}`)}
      onCopyWorkId={() => action(`Copy ${product.workId}`)}
      onCopyCredit={copyCredit}
      onShowTooltip={(text) => action(text)}
      onMoveTooltip={() => {}}
      onHideTooltip={() => {}}
      onOpenDlsite={() => action(`Open ${product.workId} on DLsite`)}
      onDownload={() => action(`${downloadLabel} ${product.workId}`)}
      onToggleMenu={() => action(`Toggle actions for ${product.workId}`)}
    />
  </section>
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    gap: 12px;
    width: min(1120px, calc(100vw - 48px));
    margin: 0 auto;
    padding: 24px 0;
  }

  .card-frame {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
    overflow: hidden;
  }

  p {
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }

  @media (max-width: 720px) {
    .story-surface {
      width: calc(100vw - 24px);
      padding: 12px 0;
    }
  }
</style>
