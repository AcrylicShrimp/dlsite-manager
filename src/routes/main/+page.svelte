<script lang="ts">
  import {
    DLsiteProductDownloadState,
    type DLsiteProductAge,
    type DLsiteProductType,
    type Product,
    type ProductDownload,
  } from "@app/types/product";
  import type {
    DownloadComplete,
    DownloadProgress,
  } from "@app/types/download-event";
  import type { RefreshProgress } from "@app/types/refresh-event";

  import Input from "@app/lib/inputs/Input.svelte";
  import LabeledSelect from "@app/lib/selects/LabeledSelect.svelte";
  import SmallButtonLink from "@app/lib/buttons/SmallButtonLink.svelte";
  import SmallFixedRedButton from "@app/lib/buttons/SmallFixedRedButton.svelte";
  import SmallFixedRedWithMenuButton from "@app/lib/buttons/SmallFixedRedWithMenuButton.svelte";
  import SmallFixedBrightRedButton from "@app/lib/buttons/SmallFixedBrightRedButton.svelte";
  import SmallFixedBrightRedWithMenuButton from "@app/lib/buttons/SmallFixedBrightRedWithMenuButton.svelte";
  import SmallMenuButton from "@app/lib/buttons/SmallMenuButton.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import throttle from "lodash/throttle";
  import { onMount } from "svelte";

  import { BgCssAge, BgCssType, DisplayTypeString } from "./product-values";

  type Age = "" | DLsiteProductAge;
  type Type = "" | DLsiteProductType;
  type DownloadState = "" | DLsiteProductDownloadState;

  let query: string = "";
  let queryAge: Age = "";
  let queryType: Type = "";
  let queryDownloadState: DownloadState = "";
  let queryOrderBy = "desc";
  let products: Product[] = [];
  let productDownloadedPaths: Map<string, string> = new Map();
  let productDownloadProgresses: Map<string, [number, boolean]> = new Map();
  let updating: boolean = false;
  let showProgress: boolean = false;
  let progress: number = 0;
  let progressTotal: number = 0;

  onMount(async () => {
    const appWindow = getCurrentWindow();
    await appWindow.listen("refresh-begin", (event) => {
      updating = true;
      showProgress = event.payload !== "no-progress";
      progress = 0;
      progressTotal = 0;
    });
    await appWindow.listen<RefreshProgress>("refresh-progress", (event) => {
      progress = event.payload.progress;
      progressTotal = event.payload.total_progress;
    });
    await appWindow.listen("refresh-end", async () => {
      await queryProducts();
      updating = false;
    });
    await appWindow.listen<string>("download-begin", (event) => {
      productDownloadProgresses.set(event.payload, [0, false]);
      productDownloadProgresses = productDownloadProgresses;
      filterProducts(products);
    });
    await appWindow.listen<DownloadProgress>("download-progress", (event) => {
      productDownloadProgresses.set(event.payload.product_id, [
        event.payload.progress,
        event.payload.decompressing,
      ]);
      productDownloadProgresses = productDownloadProgresses;
    });
    await appWindow.listen<DownloadComplete>("download-end", (event) => {
      productDownloadedPaths.set(
        event.payload.product_id,
        event.payload.downloaded_path
      );
      productDownloadedPaths = productDownloadedPaths;
      productDownloadProgresses.delete(event.payload.product_id);
      productDownloadProgresses = productDownloadProgresses;
      filterProducts(products);
    });
    await appWindow.listen<string>("download-invalid", (event) => {
      productDownloadedPaths.delete(event.payload);
      productDownloadedPaths = productDownloadedPaths;
      filterProducts(products);
    });

    await queryProducts();
    await invoke("show_window");
  });

  const throttledSearch = throttle(search, 250, {
    leading: false,
    trailing: true,
  });
  async function search(event: Event): Promise<void> {
    query = (event.target as HTMLInputElement).value;
    await queryProducts();
  }
  async function setQueryAge(event: Event): Promise<void> {
    queryAge = (event.target as HTMLSelectElement).value as Age;
    await queryProducts();
  }
  async function setQueryType(event: Event): Promise<void> {
    queryType = (event.target as HTMLSelectElement).value as Type;
    await queryProducts();
  }
  async function setQueryDownloadState(event: Event): Promise<void> {
    queryDownloadState = (event.target as HTMLSelectElement)
      .value as DownloadState;
    await queryProducts();
  }
  async function setQueryOrderBy(event: Event): Promise<void> {
    queryOrderBy = (event.target as HTMLSelectElement).value;
    await queryProducts();
  }

  async function queryProducts(): Promise<void> {
    const productQuery = {
      query,
      ...(queryAge ? { age: queryAge } : {}),
      ...(queryType ? { ty: queryType } : {}),
      order_by_asc: queryOrderBy === "asc",
    };

    const unfilteredProducts = await invoke<Product[]>(
      "product_list_products",
      {
        query: productQuery,
      }
    );

    const productDownloads = await invoke<ProductDownload[]>(
      "product_list_product_downloads",
      {
        productIds: unfilteredProducts.map((product) => product.id),
      }
    );

    productDownloadedPaths = new Map(
      productDownloads.map((download) => [download.product_id, download.path])
    );

    filterProducts(unfilteredProducts);
  }

  function filterProducts(unfilteredProducts: Product[]): void {
    switch (queryDownloadState) {
      case DLsiteProductDownloadState.NotDownloaded:
        products = unfilteredProducts.filter(
          (product) =>
            !productDownloadedPaths.has(product.id) &&
            !productDownloadProgresses.has(product.id)
        );
        break;
      case DLsiteProductDownloadState.Downloading:
        products = unfilteredProducts.filter((product) =>
          productDownloadProgresses.has(product.id)
        );
        break;
      case DLsiteProductDownloadState.Downloaded:
        products = unfilteredProducts.filter((product) =>
          productDownloadedPaths.has(product.id)
        );
        break;
      case DLsiteProductDownloadState.DownloadingAndDownloaded:
        products = unfilteredProducts.filter(
          (product) =>
            productDownloadedPaths.has(product.id) ||
            productDownloadProgresses.has(product.id)
        );
        break;
      default:
        products = unfilteredProducts;
        break;
    }
  }

  async function requestDownload(
    product: Product,
    decompress: boolean
  ): Promise<void> {
    if (productDownloadProgresses.has(product.id)) return;

    productDownloadProgresses.set(product.id, [0, false]);
    productDownloadProgresses = productDownloadProgresses;

    await invoke("product_download_product", {
      accountId: product.account_id,
      productId: product.id,
      decompress,
    });
  }

  async function openDownloadedFolder(product: Product): Promise<void> {
    await invoke("product_open_downloaded_folder", {
      productId: product.id,
    });
  }

  async function removeDownloadedFolder(product: Product): Promise<void> {
    await invoke("product_remove_downloaded_product", {
      productId: product.id,
    });
  }
</script>

<nav class="flex items-center justify-stretch">
  <div class="flex-1" />
  <h1 class="flex-none inline-block text-lg">DLsite Manager</h1>
  <div class="flex-1 flex items-center justify-end">
    <a
      href="https://github.com/AcrylicShrimp/dlsite-manager"
      target="_blank"
      rel="noreferrer"
      class="underline text-4/5">Visit Github</a
    >
    <span class="w-4" />
    <a
      href="https://www.dlsite.com/index.html"
      target="_blank"
      rel="noreferrer"
      class="underline text-4/5">Visit DLsite</a
    >
  </div>
</nav>
<span class="block h-4" />
<section>
  <div class="flex flex-row items-center justify-start">
    <Input
      placeholder="Search anything e.g. title, group, artist"
      bind:value={query}
      on:input={throttledSearch}
    />
  </div>
  <span class="block h-2" />
  <div class="px-3 py-2 bg-1/5 rounded-lg">
    <LabeledSelect label="Age" bind:value={queryAge} on:change={setQueryAge}>
      <option value="" selected>-</option>
      <option value="All">All</option>
      <option value="R15">R15</option>
      <option value="R18">R18</option>
    </LabeledSelect>
    <span class="block h-2" />
    <LabeledSelect label="Type" bind:value={queryType} on:change={setQueryType}>
      <option value="" selected>-</option>
      <option value="Adult">Adult</option>
      <option value="Doujinsji">Doujinsji</option>
      <option value="Software">Software</option>
      <option value="Game">Game</option>
      <option value="Action">Action</option>
      <option value="Adventure">Adventure</option>
      <option value="AudioMaterial">AudioMaterial</option>
      <option value="Comic">Comic</option>
      <option value="DigitalNovel">DigitalNovel</option>
      <option value="Other">Other</option>
      <option value="OtherGame">OtherGame</option>
      <option value="Illust">Illust</option>
      <option value="ImageMaterial">ImageMaterial</option>
      <option value="Manga">Manga</option>
      <option value="Anime">Anime</option>
      <option value="Music">Music</option>
      <option value="Novel">Novel</option>
      <option value="Puzzle">Puzzle</option>
      <option value="Quiz">Quiz</option>
      <option value="RolePlaying">RolePlaying</option>
      <option value="Gekiga">Gekiga</option>
      <option value="Simulation">Simulation</option>
      <option value="Voice">Voice</option>
      <option value="Shooter">Shooter</option>
      <option value="Tabletop">Tabletop</option>
      <option value="Utility">Utility</option>
      <option value="Typing">Typing</option>
      <option value="SexualNovel">SexualNovel</option>
      <option value="VoiceComic">VoiceComic</option>
    </LabeledSelect>
    <span class="block h-2" />
    <LabeledSelect
      label="Download"
      bind:value={queryDownloadState}
      on:change={setQueryDownloadState}
    >
      <option value="" selected>-</option>
      <option value="NotDownloaded">Not Downloaded</option>
      <option value="Downloading">Downloading</option>
      <option value="Downloaded">Downloaded</option>
      <option value="DownloadingAndDownloaded"
        >Downloading and Downloaded</option
      >
    </LabeledSelect>
  </div>
  <span class="block h-2" />
  <div class="px-3 py-2 bg-1/5 rounded-lg">
    <LabeledSelect
      label="Order By"
      bind:value={queryOrderBy}
      on:change={setQueryOrderBy}
    >
      <option value="desc" selected>Descending</option>
      <option value="asc">Ascending</option>
    </LabeledSelect>
  </div>
  <span class="block h-2" />
  <div>
    {#each products as product, index (product)}
      <div class="p-2 border border-1/5 rounded">
        <div class="flex flex-row items-start justify-start">
          <img
            src={product.thumbnail}
            width="64"
            height="64"
            alt={product.title}
            class="rounded"
          />
          <span class="flex-none block w-4" />
          <div class="flex-1 min-w-0 flex flex-col items-start justify-start">
            <p
              class="text-4/5 text-lg min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap"
              title={product.title}
            >
              {product.title}
            </p>
            <span class="flex-none block h-1" />
            <a
              href={`https://www.dlsite.com/maniax/circle/profile/=/maker_id/${product.group_id}.html`}
              target="_blank"
              rel="noreferrer"
              title={product.group_name}
              class="text-3/5 text-sm min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap hover:underline"
            >
              {product.group_name}
            </a>
            <span class="flex-none block h-2" />
            <div
              class="min-w-0 max-w-full w-full flex flex-row items-center justify-start"
            >
              <span
                class={`text-sm w-8 h-[1.5em] flex flex-row items-center justify-center ${
                  BgCssAge[product.age] ?? BgCssAge.Unknown
                } rounded`}>{product.age}</span
              >
              <span class="flex-none block w-1" />
              <span
                class={`text-sm px-1 h-[1.5em] flex flex-row items-center justify-center ${
                  BgCssType[product.ty] ?? BgCssType.Unknown
                } rounded`}
                >{DisplayTypeString[product.ty] ?? `Other(${product.ty})`}</span
              >
              {#if product.account_id === null}
                <span class="flex-none block w-1" />
                <span
                  class={`text-sm px-1 h-[1.5em] flex flex-row items-center justify-center ${BgCssType.Unknown} rounded`}
                  >Not Owned</span
                >
              {/if}
              {#if product.registered_at === null}
                <span class="flex-none block w-1" />
                <span
                  class={`text-sm px-1 h-[1.5em] flex flex-row items-center justify-center ${BgCssType.Unknown} rounded`}
                  >Discontinued</span
                >
              {/if}
              <span class="flex-1" />
              <SmallButtonLink
                href={`https://www.dlsite.com/maniax/work/=/product_id/${product.id}.html`}
                rel="noreferrer">Visit Product Page</SmallButtonLink
              >
              <span class="flex-none block w-1" />
              {#if productDownloadedPaths.has(product.id)}
                {#if product.account_id === null}
                  <SmallFixedBrightRedButton
                    on:click={() => openDownloadedFolder(product)}
                  >
                    Open Folder
                  </SmallFixedBrightRedButton>
                {:else}
                  <SmallFixedBrightRedWithMenuButton
                    on:click={() => openDownloadedFolder(product)}
                  >
                    Open Folder
                    <span slot="right">...</span>
                    <div
                      slot="menu"
                      class="flex flex-col items-stretch justify-start"
                    >
                      <SmallMenuButton
                        on:click={() => openDownloadedFolder(product)}
                        >Open Folder</SmallMenuButton
                      >
                      <SmallMenuButton
                        on:click={() => removeDownloadedFolder(product)}
                        >Remove Download</SmallMenuButton
                      >
                    </div>
                  </SmallFixedBrightRedWithMenuButton>
                {/if}
              {:else if productDownloadProgresses.has(product.id)}
                <SmallFixedRedButton disabled>
                  {#if productDownloadProgresses.get(product.id)?.[1]}
                    Decompressing...
                  {:else}
                    Downloading... {productDownloadProgresses.get(
                      product.id
                    )?.[0] ?? 0}%
                  {/if}
                </SmallFixedRedButton>
              {:else}
                <SmallFixedRedWithMenuButton
                  on:click={() => requestDownload(product, true)}
                >
                  Download
                  <span slot="right">...</span>
                  <div
                    slot="menu"
                    class="flex flex-col items-stretch justify-start"
                  >
                    <SmallMenuButton
                      on:click={() => requestDownload(product, true)}
                      >Download</SmallMenuButton
                    >
                    <SmallMenuButton
                      on:click={() => requestDownload(product, false)}
                      >Download w/o Decompress</SmallMenuButton
                    >
                  </div>
                </SmallFixedRedWithMenuButton>
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
    {#if showProgress}
      <p class="text-4/5 text-xl">{progress}/{progressTotal}</p>
    {/if}
  </div>
{/if}
