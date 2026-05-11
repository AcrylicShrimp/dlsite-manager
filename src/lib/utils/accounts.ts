import type { Account } from "$lib/model/types";
import { shortDate } from "$lib/utils/format";

export function accountLoginLabel(account: Account) {
  return account.loginName?.trim() || "No login name";
}

export function accountLastSyncLabel(account: Account) {
  return account.lastSyncAt ? shortDate(account.lastSyncAt) : "Never synced";
}

export function accountCredentialLabel(account: Account) {
  return account.hasCredential ? "Saved" : "Not saved";
}

export function accountEnabledLabel(account: Account) {
  return account.enabled ? "Enabled" : "Disabled";
}

export function enabledAccountCount(accounts: Account[]) {
  return accounts.filter((account) => account.enabled).length;
}

export function credentialedAccountCount(accounts: Account[]) {
  return accounts.filter((account) => account.hasCredential).length;
}
