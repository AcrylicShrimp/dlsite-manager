// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces

import type { Account } from "./types/account";

declare global {
  interface Window {
    accountId?: number;
  }
}

// and what to do when importing types
declare namespace App {}
