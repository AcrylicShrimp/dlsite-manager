<script lang="ts">
  import type { ProductImagePreview } from "$lib/model/types";

  let {
    preview,
    onClose,
  }: {
    preview: ProductImagePreview;
    onClose?: () => void;
  } = $props();
</script>

<div
  class="image-preview"
  role="dialog"
  aria-modal="true"
  aria-labelledby="image-preview-title"
  tabindex="-1"
  onkeydown={(event) => {
    if (event.key === "Escape") onClose?.();
  }}
>
  <button
    class="backdrop"
    type="button"
    aria-label="Close image preview"
    onclick={onClose}
  ></button>
  <div class="panel">
    <div class="heading">
      <div>
        <h2 id="image-preview-title">{preview.title}</h2>
        <p>{preview.workId}</p>
      </div>
      <button class="close" type="button" aria-label="Close image preview" onclick={onClose}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>
    <div class="frame">
      <img src={preview.url} alt={`Preview of ${preview.title}`} />
    </div>
  </div>
</div>

<style>
  .image-preview {
    position: fixed;
    z-index: 90;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 28px;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 70%);
    cursor: default;
  }

  .panel {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 14px;
    width: min(920px, 92vw);
    max-height: 88vh;
    padding: 16px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
  }

  .heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  h2 {
    margin: 0;
    color: var(--text-strong);
    font-size: 17px;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  p {
    margin: 4px 0 0;
    color: var(--muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
  }

  .close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .close:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .frame {
    display: grid;
    place-items: center;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    overflow: hidden;
  }

  img {
    display: block;
    max-width: 100%;
    max-height: calc(88vh - 110px);
    object-fit: contain;
  }

  @media (max-width: 720px) {
    .image-preview {
      padding: 12px;
    }

    .panel {
      width: 100%;
      padding: 12px;
    }
  }
</style>
