import { invoke } from "@tauri-apps/api/tauri";
import type { Account } from "src/types/account";

import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ params }) => {
  return {
    account: await invoke<Account>("account_management_get_account", {
      accountId: Number(params.id),
    }),
  };
};
