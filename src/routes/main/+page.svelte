<script lang="ts">
  import type { PageData } from "./$types";
  import type { Product } from "@app/types/product";

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  let products: Product[] = [];
  export let data: PageData;

  onMount(async () => {
    products = data.products;

    await invoke("show_window");
  });

  async function update() {
    products = [];
    products = await invoke<Product[]>("product_update_products");
  }
</script>

<h1 class="text-center">Product List</h1>
<span class="block h-4" />
<section>
  <button on:click={update}>Update</button>
  <div>
    {#each products as product, index (product)}
      <div
        class="p-1 pl-2 border border-1/5 rounded flex flex-row items-center justify-start"
      >
        <p
          class="text-4/5 max-w-xs truncate flex flex-row items-center justify-center"
        >
          {product.product.title.japanese}
        </p>
        <span class="flex-1" />
      </div>
      {#if index < products.length - 1}
        <span class="block h-2" />
      {/if}
    {/each}
  </div>
</section>
