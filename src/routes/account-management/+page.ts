import { invoke } from "core";
import type { Account } from "src/types/account";

import type { PageLoad } from "./$types";

export const load: PageLoad = async () => {
  return {
    accounts: await invoke<Account[]>("account_management_list_accounts"),
  };
};
