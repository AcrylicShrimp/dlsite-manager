<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { Product, ProductCreditField } from "$lib/model/types";
  import { shortDate } from "$lib/utils/format";
  import {
    ageLabel,
    ageTone,
    ageTooltip,
    creditTooltip,
    localOnlyTooltip,
    productCreditFields,
    productIsLocalOnly,
    productType,
  } from "$lib/utils/products";

  let {
    product,
    detailLoading = false,
    downloadLabel,
    downloadTitle,
    downloadDisabled = false,
    menuOpen = false,
    onPreview,
    onOpenDetails,
    onCopyWorkId,
    onCopyCredit,
    onShowTooltip,
    onMoveTooltip,
    onHideTooltip,
    onOpenDlsite,
    onDownload,
    onToggleMenu,
  }: {
    product: Product;
    detailLoading?: boolean;
    downloadLabel: string;
    downloadTitle: string;
    downloadDisabled?: boolean;
    menuOpen?: boolean;
    onPreview: (product: Product) => void;
    onOpenDetails: (product: Product) => void;
    onCopyWorkId: (workId: string) => void;
    onCopyCredit: (field: ProductCreditField, workId: string) => void;
    onShowTooltip: (text: string, event: MouseEvent) => void;
    onMoveTooltip: (text: string, event: MouseEvent) => void;
    onHideTooltip: () => void;
    onOpenDlsite: (workId: string) => void;
    onDownload: (product: Product) => void;
    onToggleMenu: (product: Product, event: MouseEvent) => void;
  } = $props();

  let typeInfo = $derived(productType(product));
  let credits = $derived(productCreditFields(product));
</script>

<article class="product-card" data-tone={typeInfo.tone}>
  <div class="type-belt" aria-hidden="true"></div>
  {#if product.thumbnailUrl}
    <button
      class="thumb"
      type="button"
      title={`Preview ${product.title}`}
      aria-label={`Preview image for ${product.title}`}
      onclick={(event) => {
        event.stopPropagation();
        onPreview(product);
      }}
    >
      <img src={product.thumbnailUrl} alt="" loading="lazy" />
    </button>
  {:else}
    <div class="thumb" aria-hidden="true">
      <span>?</span>
    </div>
  {/if}

  <div class="product-main">
    <div class="product-title-row">
      <button
        class="product-title"
        type="button"
        title={`Open details for ${product.title}`}
        disabled={detailLoading}
        onclick={() => onOpenDetails(product)}
      >
        {product.title}
      </button>
      <button
        class="work-id"
        type="button"
        title={`Copy ${product.workId}`}
        onclick={(event) => {
          event.stopPropagation();
          onCopyWorkId(product.workId);
        }}
      >
        {product.workId}
      </button>
    </div>

    <div class="product-meta">
      {#each credits as field (field.key)}
        <button
          class="credit-row"
          type="button"
          title={creditTooltip(field)}
          aria-label={field.missing
            ? `${field.label} is not available`
            : `Copy ${field.label}: ${field.value}`}
          disabled={field.missing}
          onclick={(event) => {
            event.stopPropagation();
            onCopyCredit(field, product.workId);
          }}
        >
          <span class="credit-label">{field.label}</span>
          <span class:missing={field.missing} class="credit-value">{field.value}</span>
        </button>
      {/each}
    </div>

    <div class="labeled-row" aria-label="Classifications">
      <span class="credit-label">Tags</span>
      <div class="chip-row">
        <span
          class="chip type-chip"
          role="note"
          aria-label={typeInfo.tooltip}
          onmouseenter={(event) => onShowTooltip(typeInfo.tooltip, event)}
          onmousemove={(event) => onMoveTooltip(typeInfo.tooltip, event)}
          onmouseleave={onHideTooltip}
        >
          {typeInfo.label}
        </span>
        {#if ageLabel(product.ageCategory)}
          <span
            class="chip age-chip"
            role="note"
            data-age={ageTone(product.ageCategory)}
            aria-label={ageTooltip(product.ageCategory)}
            onmouseenter={(event) => onShowTooltip(ageTooltip(product.ageCategory), event)}
            onmousemove={(event) => onMoveTooltip(ageTooltip(product.ageCategory), event)}
            onmouseleave={onHideTooltip}
          >
            {ageLabel(product.ageCategory)}
          </span>
        {/if}
        {#if productIsLocalOnly(product)}
          <span
            class="chip source-chip source-chip--local"
            role="note"
            title={localOnlyTooltip(product.workId)}
          >
            Local Only
          </span>
        {/if}
        {#each product.customTags as tag (tag.name)}
          <span class="chip custom-tag-chip" role="note" title={`Custom tag: ${tag.name}`}>
            {tag.name}
          </span>
        {/each}
      </div>
    </div>

    <div class="product-footer">
      <div class="labeled-row owner-row" aria-label="Owners">
        <span class="credit-label">Owned by</span>
        <div class="owner-list">
          {#each product.owners as owner (owner.accountId)}
            <span
              title={owner.purchasedAt
                ? `${owner.label}: ${shortDate(owner.purchasedAt)}`
                : owner.label}
            >
              {owner.label}
            </span>
          {/each}
        </div>
      </div>
      <div class="product-actions" aria-label="Actions">
        <UiButton
          variant="secondary"
          size="small"
          responsiveWidth="auto"
          title={`Open ${product.workId} on DLsite`}
          onclick={(event) => {
            event.stopPropagation();
            onOpenDlsite(product.workId);
          }}
        >
          DLsite
        </UiButton>
        <UiButton
          size="small"
          responsiveWidth="auto"
          title={downloadTitle}
          disabled={downloadDisabled}
          onclick={(event) => {
            event.stopPropagation();
            onDownload(product);
          }}
        >
          {downloadLabel}
        </UiButton>
        <UiButton
          variant="secondary"
          size="small"
          responsiveWidth="auto"
          title="More actions"
          ariaLabel="More actions"
          ariaExpanded={menuOpen}
          onclick={(event) => {
            event.stopPropagation();
            onToggleMenu(product, event);
          }}
        >
          <span class="more-icon" aria-hidden="true">•••</span>
        </UiButton>
      </div>
    </div>
  </div>
</article>

<style>
  .product-card {
    --type-color: #6b7177;
    --type-soft: rgb(107 113 119 / 18%);
    --meta-column-gap: clamp(8px, 1.15vw, 14px);
    --credit-label-width: clamp(60px, 4.1vw, 66px);
    --credit-gap: clamp(5px, 0.7vw, 7px);
    --meta-width: min(100%, clamp(520px, 48vw, 760px));
    --meta-grid-height: 74px;
    --row-height: 220px;
    --thumb-size: 112px;

    display: grid;
    grid-template-columns: 5px var(--thumb-size) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    height: var(--row-height);
    padding: 12px 14px 12px 0;
    border-bottom: 1px solid var(--border);
    contain: layout paint;
    overflow: hidden;
    overflow-anchor: none;
  }

  .product-card:hover {
    background: var(--panel-soft);
  }

  .product-card:last-child {
    border-bottom: 0;
  }

  .product-card[data-tone="audio"] {
    --type-color: #d8a62d;
    --type-soft: rgb(216 166 45 / 17%);
  }

  .product-card[data-tone="video"] {
    --type-color: #d64b92;
    --type-soft: rgb(214 75 146 / 17%);
  }

  .product-card[data-tone="voice-comic"] {
    --type-color: #55bfe6;
    --type-soft: rgb(85 191 230 / 16%);
  }

  .product-card[data-tone="game"] {
    --type-color: #9863df;
    --type-soft: rgb(152 99 223 / 17%);
  }

  .product-card[data-tone="image"] {
    --type-color: #4fb85b;
    --type-soft: rgb(79 184 91 / 16%);
  }

  .type-belt {
    align-self: stretch;
    width: 5px;
    border-radius: 0 6px 6px 0;
    background: var(--type-color);
  }

  .thumb {
    display: block;
    width: var(--thumb-size);
    height: var(--thumb-size);
    min-width: 0;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: inherit;
    background: var(--panel-raised);
    cursor: pointer;
    overflow: hidden;
  }

  .thumb:hover {
    border-color: var(--type-color);
  }

  .thumb:focus-visible {
    border-color: var(--type-color);
    outline: 2px solid var(--type-soft);
    outline-offset: 2px;
  }

  .thumb[aria-hidden="true"] {
    cursor: default;
  }

  .thumb[aria-hidden="true"]:hover {
    border-color: var(--border-strong);
  }

  .thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb span {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--text-subtle);
    font-weight: 700;
  }

  .product-main {
    display: grid;
    grid-template-rows: auto var(--meta-grid-height) 24px 32px;
    gap: 9px;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .product-title-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
    min-width: 0;
  }

  .product-title {
    display: block;
    width: 100%;
    min-width: 0;
    min-height: 0;
    height: auto;
    padding: 0;
    border: 0;
    color: var(--text-strong);
    background: transparent;
    font: inherit;
    font-size: 17px;
    font-weight: 700;
    line-height: 1.25;
    text-align: left;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-title:hover:not(:disabled) {
    color: var(--accent);
  }

  .product-title:focus-visible,
  .work-id:focus-visible,
  .credit-row:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .work-id {
    min-width: 102px;
    height: 27px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--muted);
    background: var(--field);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }

  .work-id:hover {
    border-color: var(--type-color);
    color: var(--text);
  }

  .product-meta {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: repeat(4, 15px);
    align-content: start;
    gap: 3px var(--meta-column-gap);
    justify-self: start;
    width: var(--meta-width);
    height: var(--meta-grid-height);
    min-width: 0;
    overflow: hidden;
  }

  .credit-row,
  .labeled-row {
    display: grid;
    grid-template-columns: var(--credit-label-width) minmax(0, 1fr);
    gap: var(--credit-gap);
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.2;
  }

  .credit-row {
    width: 100%;
    height: 15px;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: transparent;
    font-family: inherit;
    cursor: pointer;
    text-align: left;
  }

  .credit-row:hover:not(:disabled) .credit-value {
    color: var(--text);
  }

  .credit-row:disabled {
    cursor: default;
    opacity: 1;
  }

  .labeled-row {
    align-items: center;
    justify-self: start;
    width: var(--meta-width);
    min-height: 24px;
  }

  .credit-label {
    color: var(--text-subtle);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .credit-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .credit-value.missing {
    color: var(--text-subtle);
    opacity: 0.72;
  }

  .chip-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
    max-height: 24px;
    overflow: hidden;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    max-width: 190px;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--muted);
    background: var(--panel-raised);
    font-size: 12px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type-chip {
    border-color: var(--type-color);
    color: var(--type-color);
    background: var(--type-soft);
  }

  .age-chip[data-age="all"] {
    border-color: rgb(112 165 120 / 58%);
    color: #9bc89f;
    background: rgb(112 165 120 / 14%);
  }

  .age-chip[data-age="r15"] {
    border-color: rgb(204 166 61 / 58%);
    color: #d2b56c;
    background: rgb(204 166 61 / 14%);
  }

  .age-chip[data-age="r18"] {
    border-color: rgb(185 64 64 / 62%);
    color: #d77b7b;
    background: rgb(185 64 64 / 16%);
  }

  .custom-tag-chip {
    border-color: rgb(96 165 250 / 54%);
    color: #9fc8ff;
    background: rgb(96 165 250 / 13%);
  }

  .source-chip--local {
    border-color: rgb(100 181 217 / 58%);
    color: #9ed8ef;
    background: rgb(100 181 217 / 13%);
  }

  .product-footer {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    height: 32px;
    min-width: 0;
  }

  .owner-list {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 5px;
    min-width: 0;
    max-height: 24px;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
  }

  .owner-list span {
    max-width: 150px;
    padding: 3px 7px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--text);
    background: var(--panel-raised);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .more-icon {
    display: block;
    min-width: 20px;
    font-size: 13px;
    letter-spacing: 0.08em;
    transform: translateY(-2px);
  }

  @media (max-width: 980px) {
    .product-card {
      --meta-column-gap: 8px;
      --credit-label-width: 62px;
      --credit-gap: 6px;
      --meta-width: 100%;
      --meta-grid-height: 148px;
      --row-height: 320px;
      --thumb-size: 84px;

      gap: 12px;
    }

    .product-meta {
      grid-template-columns: 1fr;
      grid-template-rows: repeat(7, 15px);
    }

    .product-main {
      grid-template-rows: auto var(--meta-grid-height) 24px minmax(24px, auto);
    }

    .product-footer {
      grid-template-columns: 1fr;
      gap: 8px;
      align-items: start;
      height: auto;
    }

    .product-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 720px) {
    .product-card {
      --credit-label-width: 60px;
      --credit-gap: 5px;
      --row-height: 350px;
      --thumb-size: 72px;

      padding-right: 10px;
    }

    .product-title-row {
      grid-template-columns: 1fr;
      gap: 6px;
    }

    .work-id {
      justify-self: start;
      width: auto;
    }
  }
</style>
