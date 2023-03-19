import type { PageLoad } from "./$types";
import type { LatestProductQuery } from "@app/types/latest-product-query";

import { invoke } from "@tauri-apps/api/tauri";
import type { DisplayLanguageSetting } from "@app/types/setting";

export const load: PageLoad = async () => {
  return {
    query: await invoke<LatestProductQuery>("latest_product_query_get"),
    display_language_setting: await invoke<DisplayLanguageSetting>(
      "display_language_setting_get"
    ),
  };
};
