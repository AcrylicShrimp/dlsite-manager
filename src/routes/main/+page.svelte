<script lang="ts">
  import type { PageData } from "./$types";
  import {
    DLsiteProductDownloadState,
    ProductQueryOrderBy,
    type DLsiteProductAge,
    type DLsiteProductLocalizedString,
    type DLsiteProductType,
    type Product,
  } from "@app/types/product";
  import type {
    DownloadComplete,
    DownloadProgress,
  } from "@app/types/download-event";
  import type { RefreshProgress } from "@app/types/refresh-event";
  import type { DisplayLanguageSetting } from "@app/types/setting";

  import { throttle } from "lodash";

  import { BgCssAge, BgCssType, DisplayTypeString } from "./product-values";
  import SmallButtonLink from "@app/lib/buttons/SmallButtonLink.svelte";
  import SmallFixedRedButton from "@app/lib/buttons/SmallFixedRedButton.svelte";
  import Input from "@app/lib/inputs/Input.svelte";
  import LabeledSelect from "@app/lib/selects/LabeledSelect.svelte";

  import { invoke } from "@tauri-apps/api/tauri";
  import { appWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  type Age = "" | DLsiteProductAge;
  type Type = "" | DLsiteProductType;
  type DownloadState = "" | DLsiteProductDownloadState;

  export let data: PageData;
  let query: string = "";
  let queryAge: Age = "";
  let queryType: Type = "";
  let queryDownloadState: DownloadState = "";
  let queryOrderBy = ProductQueryOrderBy.PurchaseDateDesc;
  let products: Product[] = [];
  let productDownloads: Map<string, number> = new Map();
  let updating: boolean = false;
  let progress: number = 0;
  let progressTotal: number = 0;

  onMount(async () => {
    query = data.query.query.query ?? "";
    queryAge = data.query.query.age ?? "";
    queryType = data.query.query.ty ?? "";
    queryDownloadState = data.query.download ?? "";
    queryOrderBy =
      data.query.query.order_by ?? ProductQueryOrderBy.PurchaseDateDesc;

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
        await queryProducts();
        updating = false;
      }),
      appWindow.listen<string>("download-begin", (event) => {
        productDownloads.set(event.payload, 0);
        productDownloads = productDownloads;
        filterProducts(products);
      }),
      appWindow.listen<DownloadProgress>("download-progress", (event) => {
        productDownloads.set(event.payload.product_id, event.payload.progress);
        productDownloads = productDownloads;
        filterProducts(products);
      }),
      appWindow.listen<DownloadComplete>("download-end", (event) => {
        const index = products.findIndex(
          (p) => p.product.id === event.payload.product_id
        );

        if (0 <= index) products[index].download = event.payload.download;

        productDownloads.delete(event.payload.product_id);
        productDownloads = productDownloads;
        filterProducts(products);
      }),
      appWindow.listen<string>("download-invalid", (event) => {
        const index = products.findIndex((p) => p.product.id === event.payload);

        if (index < 0) return;

        products[index].download = undefined;
        filterProducts(products);
      }),
      appWindow.listen<DisplayLanguageSetting>(
        "display-language-changed",
        (event) => {
          data.display_language_setting = event.payload;
          products = products;
        }
      ),
    ]);

    await queryProducts();
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
    queryOrderBy = (event.target as HTMLSelectElement)
      .value as ProductQueryOrderBy;
    await queryProducts();
  }

  async function queryProducts(): Promise<void> {
    const productQuery = {
      query,
      ...(queryAge ? { age: queryAge } : {}),
      ...(queryType ? { ty: queryType } : {}),
      order_by: queryOrderBy,
    };

    await invoke("latest_product_query_set", {
      query: {
        query: productQuery,
        ...(queryDownloadState ? { download: queryDownloadState } : {}),
      },
    });

    const unfilteredProducts = await invoke<Product[]>(
      "product_list_products",
      {
        query: productQuery,
      }
    );

    filterProducts(unfilteredProducts);
  }

  function filterProducts(unfilteredProducts: Product[]): void {
    switch (queryDownloadState) {
      case DLsiteProductDownloadState.NotDownloaded:
        products = unfilteredProducts.filter(
          (product) =>
            !product.download && !productDownloads.has(product.product.id)
        );
        break;
      case DLsiteProductDownloadState.Downloading:
        products = unfilteredProducts.filter((product) =>
          productDownloads.has(product.product.id)
        );
        break;
      case DLsiteProductDownloadState.Downloaded:
        products = unfilteredProducts.filter((product) => product.download);
        break;
      case DLsiteProductDownloadState.DownloadingAndDownloaded:
        products = unfilteredProducts.filter(
          (product) =>
            product.download || productDownloads.has(product.product.id)
        );
        break;
      default:
        products = unfilteredProducts;
        break;
    }
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

  function localize(text: DLsiteProductLocalizedString): string {
    for (const language of data.display_language_setting.languages) {
      const localized = text[language];
      if (localized) return localized;
    }

    return text.japanese!;
  }
</script>

<h1 class="text-center">DLsite Manager</h1>
<span class="block h-8" />
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
      <option value="IdAsc">Product Id [Ascending]</option>
      <option value="IdDesc">Product Id [Descending]</option>
      <option value="TitleAsc">Product Title [Ascending]</option>
      <option value="TitleDesc">Product Title [Descending]</option>
      <option value="GroupAsc">Product Group [Ascending]</option>
      <option value="GroupDesc">Product Group [Descending]</option>
      <option value="RegistrationDateAsc">Registration Date [Ascending]</option>
      <option value="RegistrationDateDesc"
        >Registration Date [Descending]</option
      >
      <option value="PurchaseDateAsc">Purchase Date [Ascending]</option>
      <option value="PurchaseDateDesc" selected
        >Purchase Date [Descending]</option
      >
    </LabeledSelect>
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
            alt={localize(product.product.title)}
            class="rounded"
          />
          <span class="flex-none block w-4" />
          <div class="flex-1 min-w-0 flex flex-col items-start justify-start">
            <p
              class="text-4/5 text-lg min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap"
              title={localize(product.product.title)}
            >
              {localize(product.product.title)}
            </p>
            <span class="flex-none block h-1" />
            <a
              href={`https://www.dlsite.com/maniax/circle/profile/=/maker_id/${product.product.group.id}.html`}
              target="_blank"
              rel="noreferrer"
              title={localize(product.product.group.name)}
              class="text-3/5 text-sm min-w-0 max-w-full text-ellipsis overflow-hidden whitespace-nowrap hover:underline"
            >
              {localize(product.product.group.name)}
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
