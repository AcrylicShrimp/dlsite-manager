<script lang="ts">
  import { throttle } from "lodash";

  import type { PageData } from "./$types";
  import type { Product } from "@app/types/product";
  import type { RefreshProgress } from "@app/types/refresh-event";
  import { BgCssAge, BgCssType, DisplayTypeString } from "./product-values";
  import SmallRedButton from "@app/lib/buttons/SmallRedButton.svelte";
  import SmallButtonLink from "@app/lib/buttons/SmallButtonLink.svelte";
  import Input from "@app/lib/inputs/Input.svelte";

  import { invoke } from "@tauri-apps/api/tauri";
  import { appWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  export let data: PageData;
  let query: string = "";
  let products: Product[] = [];
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
    ]);

    await invoke("show_window");

    return () => {
      for (const unlisten of unlistens) unlisten();
    };
  });

  const throttledSearch = throttle(search, 100, {
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
              <SmallRedButton>Download</SmallRedButton>
            </div>
          </div>
        </div>
      </div>
      {#if index < products.length - 1}
        <span class="block h-2" />
      {/if}
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
