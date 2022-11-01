<script lang="ts">
  import type { PageData } from "./$types";
  import type { Product } from "@app/types/product";
  import type {
    DownloadComplete,
    DownloadProgress,
  } from "@app/types/download-event";
  import type { RefreshProgress } from "@app/types/refresh-event";

  import { throttle } from "lodash";

  import { BgCssAge, BgCssType, DisplayTypeString } from "./product-values";
  import SmallButtonLink from "@app/lib/buttons/SmallButtonLink.svelte";
  import SmallFixedRedButton from "@app/lib/buttons/SmallFixedRedButton.svelte";
  import Input from "@app/lib/inputs/Input.svelte";

  import { invoke } from "@tauri-apps/api/tauri";
  import { appWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  export let data: PageData;
  let query: string = "";
  let products: Product[] = [];
  let productDownloads: Map<string, number> = new Map();
  let updating: boolean = false;
  let progress: number = 0;
  let progressTotal: number = 0;

  onMount(async () => {
    products = data.products;

    const unlistens = await Promise.all([
      appWindow.listen("refresh-begin", () => {
        updating = true;
        progress = 0;
        progressTotal = 0;
      }),
      appWindow.listen<RefreshProgress>("refresh-progress", (event) => {
        progress = event.payload.progress;
        progressTotal = event.payload.total_progress;
      }),
      appWindow.listen("refresh-end", async () => {
        await query_products();
        updating = false;
      }),
      appWindow.listen<string>("download-begin", (event) => {
        productDownloads.set(event.payload, 0);
        productDownloads = productDownloads;
      }),
      appWindow.listen<DownloadProgress>("download-progress", (event) => {
        productDownloads.set(event.payload.product_id, event.payload.progress);
        productDownloads = productDownloads;
      }),
      appWindow.listen<DownloadComplete>("download-end", (event) => {
        const index = products.findIndex(
          (p) => p.product.id === event.payload.product_id
        );

        if (0 <= index) products[index].download = event.payload.download;

        productDownloads.delete(event.payload.product_id);
        products = products;
        productDownloads = productDownloads;
      }),
      appWindow.listen<string>("download-invalid", (event) => {
        const index = products.findIndex((p) => p.product.id === event.payload);

        if (index < 0) return;

        products[index].download = undefined;
        products = products;
      }),
    ]);

    await invoke("show_window");

    return () => {
      for (const unlisten of unlistens) unlisten();
    };
  });

  const throttledSearch = throttle(search, 250, {
    leading: false,
    trailing: true,
  });
  async function search(event: Event): Promise<void> {
    query = (event.target as HTMLInputElement).value;
    await query_products();
  }

  async function query_products(): Promise<void> {
    products = await invoke<Product[]>("product_list_products", {
      query: {
        query,
      },
    });
  }

  async function requestDownload(product: Product): Promise<void> {
    if (productDownloads.has(product.product.id)) return;

    productDownloads.set(product.product.id, 0);
    await invoke("product_download_product", {
      accountId: product.account.id,
      productId: product.product.id,
    });
  }

  async function openDownloadedFolder(product: Product): Promise<void> {
    await invoke("product_open_downloaded_folder", {
      productId: product.product.id,
    });
  }
</script>

<h1 class="text-center">Product List</h1>
<span class="block h-8" />
<section>
  <div class="flex flex-row items-center justify-start">
    <Input placeholder="Search..." on:input={throttledSearch} />
  </div>
  <span class="block h-2" />
  <div>
    {#each products as product, index (product)}
      <div class="p-2 border border-1/5 rounded">
        <div class="flex flex-row items-start justify-start">
          <img
            src={product.product.icon.small}
            width="64"
            height="64"
            alt={product.product.title.japanese}
            class="rounded"
          />
          <span class="flex-none block w-4" />
          <div class="flex-1 min-w-0 flex flex-col items-start justify-start">
            <p
              class="text-4/5 text-lg min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap"
              title={product.product.title.japanese}
            >
              {product.product.title.japanese}
            </p>
            <span class="flex-none block h-1" />
            <a
              href={`https://www.dlsite.com/maniax/circle/profile/=/maker_id/${product.product.group.id}.html`}
              target="_blank"
              rel="noreferrer"
              title={product.product.group.name.japanese}
              class="text-3/5 text-sm min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap hover:underline"
            >
              {product.product.group.name.japanese}
            </a>
            <span class="flex-none block h-2" />
            <div
              class="min-w-0 max-w-full w-full flex flex-row items-center justify-start"
            >
              <span
                class={`text-sm w-8 h-[1.5em] flex flex-row items-center justify-center ${
                  BgCssAge[product.product.age]
                } rounded`}>{product.product.age}</span
              >
              <span class="flex-none block w-1" />
              <span
                class={`text-sm px-1 h-[1.5em] flex flex-row items-center justify-center ${
                  BgCssType[product.product.ty]
                } rounded`}>{DisplayTypeString[product.product.ty]}</span
              >
              <span class="flex-1" />
              <SmallButtonLink
                href={`https://www.dlsite.com/maniax/work/=/product_id/${product.product.id}.html`}
                rel="noreferrer">Visit Product Page</SmallButtonLink
              >
              <span class="flex-none block w-1" />

              {#if product.download}
                <SmallFixedRedButton
                  on:click={() => openDownloadedFolder(product)}
                >
                  Open Folder
                </SmallFixedRedButton>
              {:else if productDownloads.has(product.product.id)}
                <SmallFixedRedButton disabled>
                  {#if productDownloads.get(product.product.id)}
                    Downloading... {productDownloads.get(product.product.id)}%
                  {:else}
                    Downloading...
                  {/if}
                </SmallFixedRedButton>
              {:else}
                <SmallFixedRedButton on:click={() => requestDownload(product)}>
                  Download
                </SmallFixedRedButton>
              {/if}
            </div>
          </div>
        </div>
      </div>
      {#if index < products.length - 1}
        <span class="block h-2" />
      {/if}
    {:else}
      <div class="h-[200px] flex flex-col items-center justify-center">
        <p class="text-xl text-2/5 select-none">There's no product.</p>
        <span class="block h-4" />
        <p class="text-xl text-2/5 select-none">
          You can fetch products from the menu.
        </p>
      </div>
    {/each}
  </div>
</section>
{#if updating}
  <div
    class="fixed inset-0 bg-1/5/90 flex flex-col items-center justify-center"
  >
    <p class="text-4/5 text-xl">Updating...</p>
    <p class="text-4/5 text-xl">{progress}/{progressTotal}</p>
  </div>
{/if}
