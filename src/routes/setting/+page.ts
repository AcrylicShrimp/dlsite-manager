import type { PageLoad } from "./$types";
import type { Setting } from "@app/types/setting";

import { invoke } from "core";

export const load: PageLoad = async () => {
  return {
    setting: await invoke<Setting>("setting_get"),
  };
};
