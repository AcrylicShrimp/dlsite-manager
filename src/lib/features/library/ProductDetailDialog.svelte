<script lang="ts">
  import type { ProductDetail, JobSnapshot } from "$lib/model/types";
  import {
    ageLabel,
    productCreditFields,
    productIsLocalOnly,
    productTypeFromCode,
  } from "$lib/utils/products";
  import {
    detailDate,
    downloadStatusLabel,
    formatBytes,
  } from "$lib/utils/format";
  import { isActiveJob, jobLabel } from "$lib/utils/jobs";
  import BackToTop from "$lib/components/workspace/BackToTop.svelte";
  import Modal from "$lib/components/workspace/Modal.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import Disclosure from "$lib/components/workspace/Disclosure.svelte";

  import TagChip from "$lib/components/workspace/TagChip.svelte";
  import WorkBadges from "$lib/components/workspace/WorkBadges.svelte";
  let {
    detail,
    activeJob = null,
    customTagInput = $bindable(""),
    onClose,
    onPreview,
    onCopyText,
    onCopyWorkId,
    onCopyCredit,
    onOpenDlsite,
    onAddTags,
    onRemoveTag,
    onDownload,
    onDownloadArchives,
    onMarkDownloaded,
    onRedownload,
    onDeleteDownload,
    downloadDisabled = false,
  }: {
    detail: ProductDetail | null;
    activeJob?: JobSnapshot | null;
    customTagInput?: string;
    onClose?: () => void;
    onPreview?: (detail: ProductDetail) => void;
    onCopyText?: (label: string, value: string | null, workId: string) => void;
    onCopyWorkId?: (id: string) => void;
    onCopyCredit?: (
      field: import("$lib/model/types").ProductCreditField,
      id: string,
    ) => void;
    onOpenDlsite?: (id: string) => void;
    onAddTags?: () => void;
    onRemoveTag?: (name: string) => void;
    onDownload?: () => void;
    onDownloadArchives?: () => void;
    onMarkDownloaded?: () => void;
    onRedownload?: () => void;
    onDeleteDownload?: () => void;
    downloadDisabled?: boolean;
  } = $props();
  const job = $derived(activeJob && isActiveJob(activeJob) ? activeJob : null);
  const statusLabel = $derived(
    job
      ? job.status === "running" && job.phase === "downloading"
        ? "Downloading"
        : job.status === "running" && job.phase === "unpacking"
          ? "Unpacking"
          : jobLabel(job)
      : downloadStatusLabel(detail?.download.status ?? "notDownloaded"),
  );
  function copy(label: string, value: string | null) {
    if (detail && value) onCopyText?.(label, value, detail.workId);
  }
  function addTags(event: SubmitEvent) {
    event.preventDefault();
    onAddTags?.();
  }
</script>

{#if detail}
  <Modal title="Work details" wide onClose={() => onClose?.()}>
    <div class="product-content">
      <header class="identity-heading">
        {#if detail.thumbnailUrl}<button
            class="art dm:draft-focus-image"
            aria-label="Preview cover image"
            aria-haspopup="dialog"
            onclick={() => detail && onPreview?.(detail)}
            ><img
              src={detail.thumbnailUrl}
              alt={`${detail.title} cover`}
            /></button
          >{:else}<span class="missing-art">No image</span>{/if}
        <div>
          <WorkBadges
            workType={detail.workType}
            ageCategory={detail.ageCategory}
          />
          <h3>
            <button
              class="copy-title dm:draft-focus-text"
              title="Copy title"
              onclick={() => copy("Title", detail.title)}>{detail.title}</button
            >
          </h3>
          <button
            class="copy-value dm:draft-focus-text"
            title="Copy maker name"
            disabled={!detail.makerName}
            onclick={() => copy("Maker", detail.makerName)}
            >{detail.makerName ?? "Maker not available"}</button
          >
          <p class="quick-facts">
            {detail.contentSizeBytes === null
              ? "Size unavailable"
              : formatBytes(detail.contentSizeBytes)}<span>·</span
            >{productIsLocalOnly(detail)
              ? "Local Only"
              : detail.owners.map((owner) => owner.label).join(" · ")}
          </p>
          <div class="identity-actions">
            <Button
              variant="secondary"
              class="work-id"
              title="Copy work ID"
              onclick={() =>
                onCopyWorkId
                  ? onCopyWorkId(detail.workId)
                  : copy("Work ID", detail.workId)}
              >{detail.workId} <Icon name="copy" /></Button
            ><Button
              variant="text"
              class="text-button"
              onclick={() => detail && onOpenDlsite?.(detail.workId)}
              >Open on DLsite ↗</Button
            >
          </div>
        </div>
      </header>
      <div
        class="detail-actions dm:mt-4 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:pb-4"
      >
        <span class="dm:block dm:mb-2 dm:text-xs dm:text-draft-dim"
          >{statusLabel}</span
        >
        <div
          class="download-actions dm:flex dm:flex-wrap dm:items-center dm:gap-2"
          role="group"
          aria-label="Work actions"
        >
          <Button
            variant="primary"
            disabled={downloadDisabled ||
              Boolean(job) ||
              (detail.download.status === "downloaded" &&
                !detail.download.localPath)}
            onclick={onDownload}
            ><Icon
              name={detail.download.status === "downloaded"
                ? "folder"
                : "downloads"}
            />{detail.download.status === "downloaded"
              ? "Open folder"
              : job
                ? statusLabel
                : "Download"}</Button
          >
          {#if detail.download.status !== "downloaded"}<Button
              variant="text"
              disabled={Boolean(job)}
              onclick={onDownloadArchives}>Download archives only</Button
            ><Button
              variant="text"
              disabled={Boolean(job)}
              onclick={onMarkDownloaded}>Mark as downloaded</Button
            >{:else}<Button
              variant="text"
              disabled={Boolean(job)}
              onclick={onRedownload}>Re-download</Button
            >{/if}{#if detail.download.status !== "notDownloaded"}<Button
              variant="text"
              disabled={Boolean(job)}
              tone="error"
              onclick={onDeleteDownload}>Delete download</Button
            >{/if}
        </div>
      </div>
      {#if detail.download.errorMessage}<p class="download-error" role="status">
          <strong>Download needs attention</strong>{detail.download
            .errorMessage} ({detail.download.errorCode ?? "unknown"})
        </p>{/if}
      <div class="tag-overview">
        <section aria-labelledby="detail-custom-tags">
          <h4 id="detail-custom-tags">Custom Tags</h4>
          {#if detail.customTags.length}<div class="chips">
              {#each detail.customTags as tag}<TagChip
                  name={tag.name}
                  oncopy={() => copy("Tag", tag.name)}
                  onremove={() => onRemoveTag?.(tag.name)}
                />{/each}
            </div>{:else}<p>No custom tags</p>{/if}
          <form class="tag-form" onsubmit={addTags}>
            <input
              class="dm:draft-focus-field"
              aria-label="Add custom tags"
              placeholder="Add tags, separated by commas"
              bind:value={customTagInput}
            /><Button
              variant="secondary"
              disabled={!customTagInput.trim()}
              type="submit">Add Tag</Button
            >
          </form>
        </section>
        {#if detail.tags.length}<section aria-labelledby="detail-tags">
            <h4 id="detail-tags">DLsite tags</h4>
            <div class="chips">
              {#each detail.tags as tag}<button
                  class="dm:draft-focus-control"
                  title={tag.class}
                  onclick={() => copy("Tag", tag.name)}>{tag.name}</button
                >{/each}
            </div>
          </section>{/if}
      </div>
      <section aria-labelledby="detail-credits">
        <h4 id="detail-credits">Credits</h4>
        <dl class="credits-grid">
          {#each productCreditFields(detail) as field}<div>
              <dt>{field.label}</dt>
              <dd>
                <button
                  class="copy-value dm:draft-focus-text"
                  disabled={field.missing}
                  title={`Copy ${field.label}`}
                  onclick={() =>
                    onCopyCredit
                      ? onCopyCredit(field, detail.workId)
                      : copy(field.label, field.value)}>{field.value}</button
                >
              </dd>
            </div>{/each}
        </dl>
      </section>
      <div class="metadata">
        <Disclosure title="Download details">
          <div class="metadata-body">
            <dl>
              <dt>Status</dt>
              <dd>{downloadStatusLabel(detail.download.status)}</dd>
              <dt>Policy</dt>
              <dd>{detail.download.unpackPolicy ?? "—"}</dd>
              <dt>Local path</dt>
              <dd>
                <button
                  class="copy-value dm:draft-focus-text"
                  disabled={!detail.download.localPath}
                  onclick={() => copy("Local path", detail.download.localPath)}
                  >{detail.download.localPath ?? "—"}</button
                >
              </dd>
              <dt>Staging path</dt>
              <dd>
                <button
                  class="copy-value dm:draft-focus-text"
                  disabled={!detail.download.stagingPath}
                  onclick={() =>
                    copy("Staging path", detail.download.stagingPath)}
                  >{detail.download.stagingPath ?? "—"}</button
                >
              </dd>
              <dt>Received / total</dt>
              <dd>
                {formatBytes(detail.download.bytesReceived)} / {detail.download
                  .bytesTotal === null
                  ? "Unknown"
                  : formatBytes(detail.download.bytesTotal)}
              </dd>
              <dt>Started</dt>
              <dd>{detailDate(detail.download.startedAt)}</dd>
              <dt>Completed</dt>
              <dd>{detailDate(detail.download.completedAt)}</dd>
              {#if detail.download.errorMessage}<dt>Error</dt>
                <dd class="error">
                  {detail.download.errorMessage} ({detail.download.errorCode ??
                    "unknown"})
                </dd>{/if}
            </dl>
          </div>
        </Disclosure>
        <Disclosure title="Ownership & purchases">
          <div class="metadata-body">
            {#if productIsLocalOnly(detail)}<p>
                Local Only · Available on this device, with no linked purchase.
              </p>{:else}<ul class="owners">
                {#each detail.owners as owner}<li>
                    <span>{owner.label}</span><time
                      >{detailDate(owner.purchasedAt)}</time
                    >
                  </li>{/each}
              </ul>{/if}
          </div>
        </Disclosure>
        <Disclosure title="Work information & other names">
          <div class="metadata-body">
            <dl>
              <dt>Maker</dt>
              <dd>
                <button
                  class="copy-value dm:draft-focus-text"
                  disabled={!detail.makerName}
                  onclick={() => copy("Maker", detail.makerName)}
                  >{detail.makerName ?? "—"}</button
                >
              </dd>
              <dt>Maker ID</dt>
              <dd>
                <button
                  class="copy-value dm:draft-focus-text"
                  disabled={!detail.makerId}
                  onclick={() => copy("Maker ID", detail.makerId)}
                  >{detail.makerId ?? "—"}</button
                >
              </dd>
              <dt>Type</dt>
              <dd>
                {productTypeFromCode(detail.workType).label} · {detail.workType ??
                  "Unknown"}
              </dd>
              <dt>Age</dt>
              <dd>{ageLabel(detail.ageCategory) || "—"}</dd>
              <dt>Content size</dt>
              <dd>
                {detail.contentSizeBytes === null
                  ? "—"
                  : formatBytes(detail.contentSizeBytes)}
              </dd>
              <dt>Last detail sync</dt>
              <dd>{detailDate(detail.lastDetailSyncAt)}</dd>
            </dl>
            {#if detail.titleVariants.length || detail.makerNames.length}<Disclosure
                nested
                title="Other titles & maker names"
              >
                <dl>
                  {#each detail.titleVariants as variant}<dt>
                      Title · {variant.language}
                    </dt>
                    <dd>
                      <button
                        class="copy-value dm:draft-focus-text"
                        onclick={() => copy("Title", variant.value)}
                        >{variant.value}</button
                      >
                    </dd>{/each}{#each detail.makerNames as variant}<dt>
                      Maker · {variant.language}
                    </dt>
                    <dd>
                      <button
                        class="copy-value dm:draft-focus-text"
                        onclick={() => copy("Maker", variant.value)}
                        >{variant.value}</button
                      >
                    </dd>{/each}
                </dl>
              </Disclosure>{/if}
          </div>
        </Disclosure>
        <Disclosure title="Dates">
          <div class="metadata-body">
            <dl>
              <dt>Registered</dt>
              <dd>{detailDate(detail.registeredAt)}</dd>
              <dt>Published</dt>
              <dd>{detailDate(detail.publishedAt)}</dd>
              <dt>Updated</dt>
              <dd>{detailDate(detail.updatedAt)}</dd>
              <dt>First purchase</dt>
              <dd>{detailDate(detail.earliestPurchasedAt)}</dd>
              <dt>Latest purchase</dt>
              <dd>{detailDate(detail.latestPurchasedAt)}</dd>
            </dl>
          </div>
        </Disclosure>
      </div>
    </div>
    {#snippet actions()}<BackToTop />{/snippet}
  </Modal>
{/if}

<style>
  .product-content {
    color: var(--dm-color-draft-ink);
    font-size: 13px;
  }
  button,
  input {
    font: inherit;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 11px;
    min-height: 32px;
    border: 1px solid var(--dm-color-draft-line-strong);
    border-radius: 6px;
    background: var(--dm-color-draft-elevated);
    color: var(--dm-color-draft-ink);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .identity-heading {
    display: grid;
    grid-template-columns: 144px minmax(0, 1fr);
    gap: 20px;
    align-items: start;
  }
  .art {
    padding: 0;
    overflow: hidden;
    background: none;
  }
  .art img {
    width: 100%;
    aspect-ratio: 1;
    object-fit: contain;
    display: block;
  }
  .missing-art {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    background: var(--dm-color-draft-elevated);
    color: var(--dm-color-draft-dim);
    border-radius: 6px;
  }
  h3 {
    margin: 9px 0 7px;
  }
  h4 {
    margin: 0 0 10px;
    font-size: 14px;
    font-weight: 600;
  }
  p {
    color: var(--dm-color-draft-dim);
    line-height: 1.6;
    font-size: 12px;
    margin: 8px 0;
  }
  button.copy-title,
  button.copy-value {
    background: none;
    border: 0;
    border-radius: 0;
    padding: 0;
    min-height: 0;
    text-align: left;
    overflow-wrap: anywhere;
  }
  button.copy-title {
    font-size: 21px;
    line-height: 1.4;
    font-weight: 600;
  }
  button.copy-value {
    font-size: 12px;
    line-height: 1.7;
  }
  button.copy-value:hover:not(:disabled),
  button.copy-title:hover {
    color: var(--dm-color-draft-accent);
    text-decoration: underline;
  }
  .identity-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 12px;
  }
  section {
    padding: 18px 0;
    border-bottom: 1px solid var(--dm-color-draft-line);
  }
  dl {
    display: grid;
    grid-template-columns: 135px minmax(0, 1fr);
    gap: 11px 20px;
    margin: 0;
    font-size: 12px;
  }
  dt {
    color: var(--dm-color-draft-dim);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
    line-height: 1.7;
  }
  .owners {
    display: grid;
    gap: 12px;
    padding: 0;
    margin: 0;
    list-style: none;
  }
  .owners li {
    display: flex;
    justify-content: space-between;
    gap: 15px;
    font-size: 12px;
  }
  time {
    color: var(--dm-color-draft-dim);
  }
  .error {
    color: var(--dm-color-draft-error);
  }
  .chips {
    display: flex;
    gap: 7px;
    flex-wrap: wrap;
  }
  .chips button {
    font-size: 11px;
    overflow-wrap: anywhere;
    text-align: left;
  }
  .tag-form {
    display: flex;
    gap: 8px;
    margin-top: 14px;
  }
  input {
    flex: 1;
    min-width: 0;
    border: 1px solid
      var(--draft-field-border, var(--dm-color-draft-line-strong));
    border-radius: 6px;
    background: var(--dm-color-draft-input);
    color: var(--dm-color-draft-ink);
    padding: 10px;
    font-size: 12px;
  }
  .quick-facts {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 7px;
    font-size: 11px;
  }
  .tag-overview {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 24px;
    border-bottom: 1px solid var(--dm-color-draft-line);
  }
  .tag-overview section {
    border: 0;
    min-width: 0;
  }
  .tag-form {
    gap: 6px;
  }
  .tag-form input {
    padding: 7px 9px;
  }
  .chips > button {
    background: var(--dm-color-draft-genre-bg);
    border-color: var(--dm-color-draft-genre-border);
    color: var(--dm-color-draft-genre-text);
  }
  .credits-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px 24px;
  }
  .credits-grid > div {
    min-width: 0;
  }
  .credits-grid dt {
    font-size: 10px;
    margin-bottom: 2px;
  }
  .metadata-body > dl {
    margin-top: 0;
  }
  .download-error {
    padding: 12px;
    background: var(--dm-color-draft-error-bg);
    border-radius: 6px;
    color: var(--dm-color-draft-error);
    overflow-wrap: anywhere;
  }
  .download-error strong {
    display: block;
    margin-bottom: 4px;
  }
  @media (max-width: 620px) {
    .tag-overview {
      grid-template-columns: minmax(0, 1fr);
      gap: 0;
    }
    .tag-overview section + section {
      padding-top: 0;
    }

    .identity-heading {
      grid-template-columns: 78px minmax(0, 1fr);
      gap: 13px;
    }
    button.copy-title {
      font-size: 17px;
    }
    dl {
      grid-template-columns: 100px minmax(0, 1fr);
      gap: 10px;
    }
    .identity-actions {
      gap: 5px;
    }
    .owners li {
      flex-direction: column;
      gap: 4px;
    }
    .tag-form {
      flex-wrap: wrap;
    }
    .tag-form input {
      flex-basis: 160px;
    }
  }
</style>
