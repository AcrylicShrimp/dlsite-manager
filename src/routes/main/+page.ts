import type { PageLoad } from "./$types";
import type { Product } from "src/types/product";

import { invoke } from "@tauri-apps/api/tauri";

export const load: PageLoad = async () => {
  return {
    products: await invoke<Product[]>("product_list_products"),
  };
};
