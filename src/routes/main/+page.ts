import type { PageLoad } from "./$types";
import type { Product } from "src/types/product";
import type { LatestProductQuery } from "@app/types/latest-product-query";

import { invoke } from "@tauri-apps/api/tauri";

export const load: PageLoad = async () => {
  return {
    query: await invoke<LatestProductQuery>("latest_product_query_get"),
  };
};
