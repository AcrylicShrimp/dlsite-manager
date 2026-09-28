<script lang="ts">
  import Icon from "$lib/components/workspace/Icon.svelte";
  import Badges from "$lib/components/workspace/WorkBadges.svelte";
  import { jobLabel } from "$lib/utils/jobs";
  import type { Product, JobSnapshot } from "$lib/model/types";
  let {
    product,
    activeJob = null,
    detailLoading = false,
    onOpenDetails,
  }: {
    product: Product;
    activeJob?: JobSnapshot | null;
    detailLoading?: boolean;
    onOpenDetails: (product: Product) => void;
  } = $props();
  const status = $derived(
    activeJob
      ? activeJob.status === "queued"
        ? { icon: "clock", label: "Queued" }
        : activeJob.status === "cancelling"
          ? { icon: "downloads", label: "Cancelling" }
          : activeJob.phase === "unpacking"
            ? { icon: "unpack", label: "Unpacking" }
            : { icon: "downloads", label: jobLabel(activeJob) }
      : product.download.status === "downloaded" && product.download.localPath
        ? { icon: "folder", label: "Available locally" }
        : product.download.status === "downloading"
          ? { icon: "downloads", label: "Downloading" }
          : null,
  );
</script>

<article class="product-card dm:min-w-0">
  <button
    type="button"
    class="dm:relative dm:block dm:aspect-[3/2] dm:w-full dm:overflow-hidden dm:rounded-lg dm:border-0 dm:bg-draft-elevated dm:p-0 dm:text-draft-ink dm:cursor-pointer dm:draft-focus-image"
    disabled={detailLoading}
    aria-label={`Details: ${product.title}`}
    onclick={() => onOpenDetails(product)}
  >
    {#if product.thumbnailUrl}<img
        src={product.thumbnailUrl}
        alt=""
        loading="lazy"
        class="dm:block dm:h-full dm:w-full dm:object-cover"
      />{:else}<span class="dm:text-sm dm:text-draft-dim">No cover image</span
      >{/if}
    <span class="dm:absolute dm:bottom-2 dm:left-2 dm:right-2 dm:flex"
      ><Badges
        workType={product.workType}
        ageCategory={product.ageCategory}
      /></span
    >
    {#if status}<span
        role="img"
        aria-label={status.label}
        title={status.label}
        class="dm:absolute dm:top-2 dm:right-2 dm:grid dm:size-7 dm:place-items-center dm:rounded-full dm:bg-draft-status-bg dm:text-draft-accent"
        ><Icon name={status.icon} /></span
      >{/if}
    {#if detailLoading}<span
        class="dm:absolute dm:inset-0 dm:grid dm:place-items-center dm:bg-draft-backdrop"
        >Loading…</span
      >{/if}
  </button>
  <button
    type="button"
    disabled={detailLoading}
    onclick={() => onOpenDetails(product)}
    class="dm:mt-3 dm:w-full dm:wrap-anywhere dm:border-0 dm:bg-transparent dm:p-0 dm:text-left dm:font-[inherit] dm:text-base dm:font-semibold dm:leading-relaxed dm:text-draft-ink dm:cursor-pointer dm:draft-focus-text"
    >{product.title}</button
  >
  <p class="dm:mb-0 dm:mt-1 dm:wrap-anywhere dm:text-sm dm:text-draft-dim">
    {product.makerName ?? "Unknown maker"}
  </p>
</article>
