<script lang="ts">
  import Field from "$lib/components/ui/Field.svelte";
  import UiButton from "$lib/components/ui/Button.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";

  let libraryRoot = $state("");
  let search = $state("voice drama");

  const swatches = [
    { name: "Background", token: "--bg" },
    { name: "Panel", token: "--panel" },
    { name: "Raised", token: "--panel-raised" },
    { name: "Field", token: "--field" },
    { name: "Accent", token: "--accent" },
    { name: "Danger", token: "--danger" },
  ];
</script>

<main class="gallery">
  <header>
    <p>dlsite-manager</p>
    <h1>Component foundations</h1>
    <span>Shared tokens and controls rendered outside the Tauri runtime.</span>
  </header>

  <section>
    <div class="section-heading">
      <h2>Color tokens</h2>
      <span>Dark desktop surfaces with restrained semantic accents.</span>
    </div>
    <div class="swatch-grid">
      {#each swatches as swatch}
        <article class="swatch">
          <div class="swatch-color" style:background={`var(${swatch.token})`}></div>
          <strong>{swatch.name}</strong>
          <code>{swatch.token}</code>
        </article>
      {/each}
    </div>
  </section>

  <section>
    <div class="section-heading">
      <h2>Actions</h2>
      <span>Primary, secondary, destructive, compact, and disabled states.</span>
    </div>
    <div class="action-row">
      <UiButton>Sync library</UiButton>
      <UiButton variant="secondary">Reload</UiButton>
      <UiButton variant="danger">Delete local files</UiButton>
      <UiButton size="small" variant="secondary">Copy ID</UiButton>
      <UiButton disabled>Downloading</UiButton>
    </div>
  </section>

  <section>
    <div class="section-heading">
      <h2>Fields</h2>
      <span>Labels and help remain legible without competing with input values.</span>
    </div>
    <div class="field-grid">
      <Field id="gallery-search" label="Search library" help="Title, maker, work ID, or custom tag">
        <TextInput id="gallery-search" type="search" bind:value={search} />
      </Field>
      <Field
        id="gallery-library-root"
        label="Library folder"
        help="Downloaded works are finalized under this folder."
      >
        <TextInput
          id="gallery-library-root"
          bind:value={libraryRoot}
          placeholder="/Users/example/Library"
        />
      </Field>
    </div>
  </section>
</main>

<style>
  .gallery {
    display: grid;
    gap: 18px;
    width: min(960px, calc(100vw - 48px));
    padding: 28px;
  }

  header,
  section {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
  }

  header {
    padding: 24px;
  }

  header p,
  header h1,
  header span,
  .section-heading h2,
  .section-heading span {
    margin: 0;
  }

  header p {
    color: var(--accent);
    font-size: 12px;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  header h1 {
    margin-top: 5px;
    color: var(--text-strong);
    font-size: clamp(24px, 4vw, 34px);
    line-height: 1.1;
  }

  header span,
  .section-heading span {
    color: var(--muted);
  }

  header span {
    display: block;
    margin-top: 8px;
  }

  section {
    display: grid;
    gap: 16px;
    padding: 20px;
  }

  .section-heading {
    display: grid;
    gap: 4px;
  }

  .section-heading h2 {
    color: var(--text-strong);
    font-size: 16px;
  }

  .section-heading span {
    font-size: 13px;
  }

  .swatch-grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 10px;
  }

  .swatch {
    display: grid;
    gap: 6px;
    min-width: 0;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .swatch-color {
    height: 54px;
    border: 1px solid rgb(255 255 255 / 10%);
    border-radius: 5px;
  }

  .swatch strong {
    font-size: 12px;
  }

  .swatch code {
    color: var(--text-subtle);
    font-size: 11px;
    overflow-wrap: anywhere;
  }

  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  @media (max-width: 720px) {
    .gallery {
      width: 100vw;
      padding: 12px;
    }

    .swatch-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
