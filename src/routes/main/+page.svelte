<script lang="ts">
  import type { PageData } from "./$types";
  import type { Product } from "@app/types/product";

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";
  import { BgCssAge, BgCssType, DisplayTypeString } from "./product-values";
  import SmallButton from "@app/lib/buttons/SmallButton.svelte";
  import SmallRedButton from "@app/lib/buttons/SmallRedButton.svelte";
  import SmallButtonLink from "@app/lib/buttons/SmallButtonLink.svelte";

  let products: Product[] = [];
  export let data: PageData;

  onMount(async () => {
    products = data.products;

    await invoke("show_window");
  });

  async function update(): Promise<void> {
    products = [];
    products = await invoke<Product[]>("product_update_products");
  }

  function visitProductPage(product: Product): void {}
</script>

<h1 class="text-center">Product List</h1>
<span class="block h-4" />
<section>
  <button on:click={update}>Update</button>
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
