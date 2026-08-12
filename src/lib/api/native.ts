import { getIdentifier, getName, getTauriVersion, getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { downloadDir } from "@tauri-apps/api/path";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppInfo, JobEvent } from "$lib/model/types";

export type DirectorySelection = {
  title: string;
  defaultPath?: string;
  canCreateDirectories: boolean;
};

export async function getAppInfo(): Promise<AppInfo> {
  const [name, version, identifier, tauriVersion] = await Promise.all([
    getName(),
    getVersion(),
    getIdentifier(),
    getTauriVersion(),
  ]);

  return { name, version, identifier, tauriVersion };
}

export function getSystemDownloadDirectory() {
  return downloadDir();
}

export function chooseDirectory(options: DirectorySelection) {
  return openDialog({
    directory: true,
    multiple: false,
    canCreateDirectories: options.canCreateDirectories,
    defaultPath: options.defaultPath,
    title: options.title,
  });
}

export function openExternalUrl(url: string) {
  return openUrl(url);
}

export function listenToJobEvents(handler: (event: JobEvent) => void) {
  return listen<JobEvent>("dm-job-event", (event) => handler(event.payload));
}
