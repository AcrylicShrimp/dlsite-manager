import type { PageLoad } from "./$types";
import type { DisplayLanguageSetting, Setting } from "@app/types/setting";

import { invoke } from "@tauri-apps/api/tauri";

export const load: PageLoad = async () => {
  return {
    setting: await invoke<Setting>("setting_get"),
    display_language_setting: await invoke<DisplayLanguageSetting>(
      "display_language_setting_get"
    ),
  };
};
