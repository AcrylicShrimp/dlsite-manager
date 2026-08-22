<script lang="ts">
  import ProductImagePreview from "$lib/features/library/ProductImagePreview.svelte";

  let { longTitle = false }: { longTitle?: boolean } = $props();

  const image =
    "data:image/svg+xml," +
    encodeURIComponent(`
      <svg xmlns="http://www.w3.org/2000/svg" width="960" height="640" viewBox="0 0 960 640">
        <defs>
          <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
            <stop stop-color="#23342d"/><stop offset="1" stop-color="#101416"/>
          </linearGradient>
        </defs>
        <rect width="960" height="640" fill="url(#g)"/>
        <circle cx="480" cy="280" r="154" fill="#95c29b" fill-opacity=".12"/>
        <path d="M290 402c74-130 306-130 380 0" fill="none" stroke="#95c29b" stroke-width="22" stroke-linecap="round"/>
        <text x="480" y="535" text-anchor="middle" fill="#edf2f6" font-family="system-ui" font-size="46" font-weight="700">RJ SAMPLE</text>
      </svg>
    `);

  let lastAction = $state("Preview open");
</script>

<ProductImagePreview
  preview={{
    url: image,
    workId: "RJ01553954",
    title: longTitle
      ? "A Very Long Product Title That Demonstrates Safe Truncation In A Narrow Preview Dialog"
      : "A Long Evening at the Observatory",
  }}
  onClose={() => (lastAction = "Close preview")}
/>

<p aria-live="polite">{lastAction}</p>

<style>
  p {
    position: fixed;
    z-index: 100;
    right: 14px;
    bottom: 10px;
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
  }
</style>
