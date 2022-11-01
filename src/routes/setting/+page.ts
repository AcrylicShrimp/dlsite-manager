import type { PageLoad } from "./$types";
import type { Setting } from "@app/types/setting";

import { invoke } from "@tauri-apps/api/tauri";

export const load: PageLoad = async () => {
  return {
    setting: await invoke<Setting>("setting_get"),
  };
};
