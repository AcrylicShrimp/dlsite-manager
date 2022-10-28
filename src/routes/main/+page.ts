import { invoke } from "@tauri-apps/api/tauri";
import type { Product } from "src/types/product";

import type { PageLoad } from "./$types";

export const load: PageLoad = async () => {
  return {
    products: await invoke<Product[]>("product_list_products"),
  };
};
