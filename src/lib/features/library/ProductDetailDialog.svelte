<script lang="ts">
  import type { ProductCreditField, ProductDetail } from "$lib/model/types";
  import {
    detailDate,
    detailValue,
    downloadStatusLabel,
    formatBytes,
    shortDate,
    textVariantsLabel,
  } from "$lib/utils/format";
  import {
    ageLabel,
    creditTooltip,
    localOnlyTooltip,
    productCreditFields,
    productIsLocalOnly,
    productTypeFromCode,
  } from "$lib/utils/products";

  let {
    detail,
    customTagInput = $bindable(""),
    onClose,
    onPreview,
    onCopyText,
    onCopyWorkId,
    onCopyCredit,
    onOpenDlsite,
    onAddTags,
    onRemoveTag,
  }: {
    detail: ProductDetail | null;
    customTagInput?: string;
    onClose?: () => void;
    onPreview?: (detail: ProductDetail) => void;
    onCopyText?: (label: string, value: string | null, workId: string) => void;
    onCopyWorkId?: (workId: string) => void;
    onCopyCredit?: (field: ProductCreditField, workId: string) => void;
    onOpenDlsite?: (workId: string) => void;
    onAddTags?: () => void;
    onRemoveTag?: (name: string) => void;
  } = $props();

  function submitTags(event: SubmitEvent) {
    event.preventDefault();
    onAddTags?.();
  }
</script>

{#if detail}
  {@const typeInfo = productTypeFromCode(detail.workType)}
  <div
    class="product-detail"
    role="dialog"
    aria-modal="true"
    aria-labelledby="product-detail-title"
    tabindex="-1"
    onkeydown={(event) => {
      if (event.key === "Escape") onClose?.();
    }}
  >
    <button class="backdrop" type="button" aria-label="Close product detail" onclick={onClose}></button>
    <section class="panel" data-tone={typeInfo.tone}>
      <div class="belt" aria-hidden="true"></div>
      <div class="heading">
        {#if detail.thumbnailUrl}
          <button
            class="thumb"
            type="button"
            aria-label={`Preview image for ${detail.title}`}
            onclick={() => onPreview?.(detail)}
          >
            <img src={detail.thumbnailUrl} alt="" />
          </button>
        {:else}
          <div class="thumb missing-thumb" aria-hidden="true">?</div>
        {/if}

        <div class="title-block">
          <p>Product detail</p>
          <button
            id="product-detail-title"
            class="title-copy"
            type="button"
            title={detail.titleVariants.length > 0
              ? textVariantsLabel(detail.titleVariants)
              : `Copy ${detail.title}`}
            onclick={() => onCopyText?.("title", detail.title, detail.workId)}
          >
            {detail.title}
          </button>
          <button class="link-button" type="button" onclick={() => onOpenDlsite?.(detail.workId)}>
            Open on DLsite
          </button>
        </div>

        <button
          class="work-id"
          type="button"
          title={`Copy ${detail.workId}`}
          onclick={() => onCopyWorkId?.(detail.workId)}
        >
          {detail.workId}
        </button>

        <button class="close" type="button" aria-label="Close product detail" onclick={onClose}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div class="body">
        <div class="column">
          <section class="detail-section">
            <h3>Identity</h3>
            <div class="detail-grid">
              <div>
                <span>Maker</span>
                <button type="button" onclick={() => onCopyText?.("maker name", detail.makerName, detail.workId)}>
                  {detailValue(detail.makerName)}
                </button>
              </div>
              <div>
                <span>Maker ID</span>
                <button type="button" onclick={() => onCopyText?.("maker ID", detail.makerId, detail.workId)}>
                  {detailValue(detail.makerId)}
                </button>
              </div>
              <div><span>Type</span><span>{typeInfo.label}</span></div>
              <div><span>Age</span><span>{ageLabel(detail.ageCategory) || "-"}</span></div>
              <div><span>Size</span><span>{detail.contentSizeBytes ? formatBytes(detail.contentSizeBytes) : "-"}</span></div>
              <div><span>Last detail sync</span><span>{detailDate(detail.lastDetailSyncAt)}</span></div>
            </div>
          </section>

          <section class="detail-section">
            <h3>Credits</h3>
            <div class="credit-list">
              {#each productCreditFields(detail) as field (field.key)}
                <button
                  type="button"
                  disabled={field.missing}
                  title={creditTooltip(field)}
                  onclick={() => onCopyCredit?.(field, detail.workId)}
                >
                  <span>{field.label}</span>
                  <strong class:missing={field.missing}>{field.value}</strong>
                </button>
              {/each}
            </div>
          </section>

          <section class="detail-section">
            <h3>Dates</h3>
            <div class="detail-grid">
              <div><span>Registered</span><span>{detailDate(detail.registeredAt)}</span></div>
              <div><span>Published</span><span>{detailDate(detail.publishedAt)}</span></div>
              <div><span>Updated</span><span>{detailDate(detail.updatedAt)}</span></div>
              <div><span>Latest Purchase</span><span>{detailDate(detail.latestPurchasedAt)}</span></div>
            </div>
          </section>
        </div>

        <div class="column">
          <section class="detail-section">
            <h3>Ownership</h3>
            <div class="chip-list">
              {#if productIsLocalOnly(detail)}
                <span class="source-local" title={localOnlyTooltip(detail.workId)}>Local Only</span>
              {:else}
                {#each detail.owners as owner (owner.accountId)}
                  <span title={owner.purchasedAt ? `${owner.label}: ${shortDate(owner.purchasedAt)}` : owner.label}>
                    {owner.label}
                  </span>
                {/each}
              {/if}
            </div>
          </section>

          <section class="detail-section">
            <h3>Download</h3>
            <div class="detail-grid">
              <div><span>Status</span><span>{downloadStatusLabel(detail.download.status)}</span></div>
              <div><span>Policy</span><span>{detailValue(detail.download.unpackPolicy)}</span></div>
              <div class="wide">
                <span>Local path</span>
                <button type="button" onclick={() => onCopyText?.("local path", detail.download.localPath, detail.workId)}>
                  {detailValue(detail.download.localPath)}
                </button>
              </div>
              {#if detail.download.errorMessage}
                <div class="wide"><span>Error</span><span>{detail.download.errorMessage}</span></div>
              {/if}
            </div>
          </section>

          <section class="detail-section">
            <h3>Custom Tags</h3>
            {#if detail.customTags.length > 0}
              <div class="chip-list custom-tag-list">
                {#each detail.customTags as tag (tag.name)}
                  <span class="custom-tag" title={`Custom tag: ${tag.name}`}>
                    <button type="button" onclick={() => onCopyText?.("custom tag", tag.name, detail.workId)}>
                      {tag.name}
                    </button>
                    <button
                      class="tag-remove"
                      type="button"
                      aria-label={`Remove custom tag ${tag.name}`}
                      title={`Remove ${tag.name}`}
                      onclick={() => onRemoveTag?.(tag.name)}
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6 6 18M6 6l12 12" /></svg>
                    </button>
                  </span>
                {/each}
              </div>
            {:else}
              <p class="muted">No custom tags</p>
            {/if}

            <form class="tag-form" onsubmit={submitTags}>
              <input
                type="text"
                autocomplete="off"
                placeholder="Add custom tags"
                aria-label="Add custom tags"
                bind:value={customTagInput}
              />
              <button type="submit" disabled={!customTagInput.trim()}>Add Tag</button>
            </form>
          </section>
        </div>
      </div>
    </section>
  </div>
{/if}

<style>
  button, input { font: inherit; }
  button { display: inline-flex; align-items: center; justify-content: center; min-width: 84px; height: 38px; padding: 0 13px; border: 1px solid var(--accent); border-radius: 6px; color: #09110c; background: var(--accent); cursor: pointer; }
  button:disabled { cursor: default; opacity: .58; }
  button:focus-visible, input:focus-visible { outline: none; box-shadow: var(--focus-ring); }

  .product-detail { position: fixed; z-index: 45; inset: 0; display: grid; place-items: center; padding: 28px; }
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; min-width: 0; padding: 0; border: 0; border-radius: 0; background: rgb(0 0 0 / 70%); cursor: default; }
  .panel { --type-color:#6b7177; --type-soft:rgb(107 113 119 / 18%); position: relative; z-index: 1; display: grid; grid-template-columns: 5px minmax(0,1fr); width:min(980px,94vw); max-height:90vh; border:1px solid var(--border-strong); border-radius:8px; background:var(--panel); box-shadow:0 24px 64px rgb(0 0 0 / 52%); overflow:hidden; }
  .panel[data-tone="audio"] { --type-color:#d8a62d; --type-soft:rgb(216 166 45 / 17%); }
  .panel[data-tone="video"] { --type-color:#d64b92; --type-soft:rgb(214 75 146 / 17%); }
  .panel[data-tone="voice-comic"] { --type-color:#55bfe6; --type-soft:rgb(85 191 230 / 16%); }
  .panel[data-tone="game"] { --type-color:#9863df; --type-soft:rgb(152 99 223 / 17%); }
  .panel[data-tone="image"] { --type-color:#4fb85b; --type-soft:rgb(79 184 91 / 16%); }
  .belt { grid-row:1 / 3; background:var(--type-color); }
  .heading { display:grid; grid-template-columns:120px minmax(0,1fr) auto auto; gap:14px; align-items:start; min-width:0; padding:16px; border-bottom:1px solid var(--border); }
  .thumb { width:120px; height:120px; min-width:0; padding:0; border-color:var(--border-strong); border-radius:6px; background:var(--panel-raised); overflow:hidden; }
  .thumb:hover { border-color:var(--type-color); }
  .thumb img { display:block; width:100%; height:100%; object-fit:cover; }
  .missing-thumb { display:grid; place-items:center; color:var(--text-subtle); font-weight:700; }
  .title-block { display:grid; align-content:start; gap:6px; min-width:0; }
  .title-block p { margin:0; color:var(--muted); font-size:12px; font-weight:700; text-transform:uppercase; }
  .title-copy { display:block; width:100%; height:auto; min-width:0; min-height:0; padding:0; border:0; color:var(--text-strong); background:transparent; font-size:22px; font-weight:700; line-height:1.24; text-align:left; overflow-wrap:anywhere; }
  .title-copy:hover { color:var(--accent); }
  .link-button { justify-self:start; min-height:26px; padding:0; border:0; color:var(--accent); background:transparent; font-size:12px; font-weight:650; }
  .link-button:hover { color:var(--text-strong); }
  .work-id { align-self:start; min-width:0; height:28px; padding:0 8px; border-color:var(--border-strong); color:var(--text); background:var(--panel-raised); font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono",monospace; font-size:12px; }
  .close { width:34px; min-width:34px; height:34px; padding:0; border-color:var(--border-strong); color:var(--muted); background:var(--panel-raised); }
  .close:hover { border-color:var(--accent); color:var(--text); }
  .close svg, .tag-remove svg { fill:none; stroke:currentColor; stroke-linecap:round; stroke-linejoin:round; stroke-width:2.35; }
  .close svg { width:18px; height:18px; }

  .body { display:grid; grid-template-columns:minmax(0,.95fr) minmax(0,1.05fr); align-items:start; gap:12px; min-height:0; max-height:calc(90vh - 154px); padding:16px; overflow:auto; }
  .column { display:grid; align-content:start; gap:12px; min-width:0; }
  .detail-section { min-width:0; padding:12px; border:1px solid var(--border); border-radius:8px; background:var(--panel-soft); }
  .detail-section h3 { margin:0 0 10px; color:var(--text-strong); font-size:13px; font-weight:700; }
  .detail-grid { display:grid; grid-template-columns:repeat(2,minmax(132px,1fr)); gap:12px; }
  .detail-grid div { display:grid; align-content:start; gap:4px; min-width:0; }
  .detail-grid .wide { grid-column:1 / -1; }
  .detail-grid span:first-child, .credit-list span { color:var(--text-subtle); font-size:12px; font-weight:700; }
  .detail-grid span:last-child, .detail-grid button, .credit-list strong { min-width:0; color:var(--text); font-size:13px; font-weight:600; line-height:1.35; overflow-wrap:anywhere; }
  .detail-grid button, .credit-list button { height:auto; min-height:0; min-width:0; padding:0; border:0; border-radius:3px; color:inherit; background:transparent; text-align:left; }
  .detail-grid button { justify-self:start; max-width:100%; }
  .detail-grid button:hover, .credit-list button:hover:not(:disabled) strong { color:var(--text-strong); }
  .credit-list { display:grid; gap:12px; }
  .credit-list button { display:grid; grid-template-columns:104px minmax(0,1fr); gap:8px; align-items:baseline; width:100%; }
  .credit-list strong.missing { color:var(--text-subtle); opacity:.72; }

  .chip-list { display:flex; flex-wrap:wrap; gap:6px; min-width:0; }
  .chip-list > span { max-width:100%; padding:4px 8px; border:1px solid var(--border-strong); border-radius:999px; color:var(--text); background:var(--panel-raised); font-size:12px; font-weight:650; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .chip-list .source-local { border-color:rgb(100 181 217 / 58%); color:#9ed8ef; background:rgb(100 181 217 / 13%); }
  .chip-list .custom-tag { display:inline-flex; align-items:center; gap:5px; max-width:100%; min-height:24px; padding:2px 6px 2px 8px; }
  .custom-tag button { width:auto; height:18px; min-width:0; min-height:0; padding:0; border:0; color:inherit; background:transparent; font-size:12px; font-weight:650; line-height:1; }
  .custom-tag button:first-child { display:block; max-width:220px; height:auto; line-height:1.2; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .custom-tag button:hover { color:var(--text-strong); }
  .tag-remove { display:grid; place-items:center; width:18px; min-width:18px; color:var(--muted); }
  .tag-remove svg { width:13px; height:13px; }
  .tag-form { display:grid; grid-template-columns:minmax(0,1fr) auto; gap:8px; margin-top:10px; }
  .tag-form input { width:100%; min-width:0; height:32px; padding:0 10px; border:1px solid var(--border-strong); border-radius:6px; color:var(--text); background:var(--field); font-size:13px; }
  .tag-form button { min-width:72px; height:32px; font-size:13px; }
  .muted { margin:0; color:var(--muted); font-size:13px; }

  @media (max-width:980px) {
    .heading { grid-template-columns:86px minmax(0,1fr) auto; gap:12px; }
    .thumb { width:86px; height:86px; }
    .work-id { grid-column:2; grid-row:2; justify-self:start; }
    .body { grid-template-columns:1fr; max-height:calc(90vh - 124px); }
  }
  @media (max-width:720px) {
    .product-detail { padding:12px; }
    .panel { width:100%; max-height:calc(100vh - 24px); }
    .heading { grid-template-columns:72px minmax(0,1fr) auto; padding:12px; }
    .title-copy { font-size:18px; }
    .thumb { width:72px; height:72px; }
    .body { max-height:calc(100vh - 116px); padding:12px; }
    .detail-grid { grid-template-columns:1fr; }
    .credit-list button { grid-template-columns:82px minmax(0,1fr); }
    .tag-form { grid-template-columns:1fr; }
    .tag-form button { width:100%; }
  }
</style>
