import type { AuditEvent, JobSnapshot, Product, View } from "$lib/model/types";
import { downloadQueueProgressPercent } from "$lib/utils/jobs";
import {
  primaryAccount,
  syncingAccount,
  disabledAccount,
  accountSyncJob,
} from "./accounts";
import {
  runningDownloadJob,
  queuedDownloadJob,
  failedDownloadJob,
} from "./jobs";

export const scenarios = [
  "populated",
  "long",
  "empty",
  "mfa",
  "mfa-rejected",
  "mfa-submitting",
] as const;
export type Scenario = (typeof scenarios)[number];
export const views: View[] = [
  "library",
  "downloads",
  "activity",
  "accounts",
  "settings",
];
export const labels: Record<View, string> = {
  library: "Library",
  downloads: "Downloads",
  activity: "Activity",
  accounts: "Accounts",
  settings: "Settings",
};
const titles = [
  "A Long Evening at the Observatory",
  "Letters from the Quiet Coast",
  "The Garden After Rain",
  "Midnight Radio — Volume 02",
  "A Small Book of Distant Places",
  "Summer, Somewhere",
  "The Last Train Home",
  "Sketches of an Ordinary Day",
  "Under the Same Moon",
  "A Room Full of Records",
  "Afterglow: Original Soundtrack",
  "Stories from the North Window",
];
const palettes = [
  ["#263d4c", "#a6c7d1"],
  ["#53483f", "#e3c7a4"],
  ["#2d493f", "#adceb3"],
  ["#42364f", "#cbb5dc"],
  ["#4a3936", "#dcae99"],
  ["#44482d", "#d2d0a4"],
];

// Synthetic cover art: identical for both layouts, no external assets or accounts.
function cover(i: number) {
  const [bg, ink] = palettes[i % palettes.length];
  return (
    "data:image/svg+xml," +
    encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="320" viewBox="0 0 480 320"><rect width="480" height="320" fill="${bg}"/><circle cx="${330 - (i % 3) * 80}" cy="100" r="65" fill="${ink}" opacity=".7"/><path d="M0 260 Q120 ${60 + (i % 4) * 35} 260 220 T480 150 V320 H0Z" fill="${ink}" opacity=".2"/><path d="M0 290 Q180 120 330 250 T480 210" fill="none" stroke="${ink}" opacity=".45"/><text x="30" y="42" fill="${ink}" font-family="sans-serif" font-size="12" letter-spacing="4">NORTH WINDOW / COLLECTION</text><text x="28" y="288" fill="${ink}" font-family="serif" font-size="46">${String(i + 1).padStart(2, "0")}</text></svg>`,
    )
  );
}

export function makeFixture(scenario: Scenario) {
  const products: Product[] = Array.from(
    { length: scenario === "empty" ? 0 : scenario === "long" ? 72 : 18 },
    (_, i) => ({
      workId: `RJ${String(15000000 + i)}`,
      title:
        titles[i % titles.length] +
        (i >= 12 ? ` · Edition ${Math.floor(i / 12) + 1}` : ""),
      makerName:
        i % 12 === 11
          ? null
          : ["North Window Studio", "Quiet Coast", "Paper Lantern"][i % 3],
      workType: ["SOU", "COM", "MUS", "MOV", "RPG", "SOF"][i % 6],
      ageCategory: ["all", "r15", "r18"][i % 3],
      thumbnailUrl: i % 12 === 11 ? null : cover(i),
      publishedAt:
        i % 12 === 11
          ? null
          : `2026-08-${String(28 - (i % 28)).padStart(2, "0")}T00:00:00Z`,
      updatedAt: "2026-09-27T00:00:00Z",
      earliestPurchasedAt: "2026-09-20T00:00:00Z",
      latestPurchasedAt:
        i % 12 === 11
          ? null
          : `2026-09-${String(28 - (i % 28)).padStart(2, "0")}T00:00:00Z`,
      creditGroups:
        i % 12 === 11
          ? []
          : [
              {
                kind: "voice",
                label: "Voice",
                names: ["Akari Example", "Yui Example"],
              },
              { kind: "illust", label: "Illustration", names: ["Studio K"] },
              { kind: "scenario", label: "Scenario", names: ["M. Hoshino"] },
              {
                kind: "creator",
                label: "Creator",
                names: ["North Window Team"],
              },
              { kind: "music", label: "Music", names: ["Night Signal"] },
              {
                kind: "other",
                label: "Other",
                names: ["Sound editing: Quiet Coast"],
              },
            ],
      customTags: [
        { name: ["Favorites", "Sleep", "Needs Review"][i % 3] },
        ...(i % 4 === 0 ? [{ name: "Long-form listening" }] : []),
      ],
      owners:
        i % 12 === 11
          ? [{ accountId: "__local__", label: "Local", purchasedAt: null }]
          : [
              {
                accountId: i % 2 === 0 ? primaryAccount.id : syncingAccount.id,
                label:
                  i % 2 === 0
                    ? "Primary DLsite account"
                    : "Secondary purchases",
                purchasedAt: "2026-09-20T08:30:00Z",
              },
              ...(i % 4 === 0
                ? [
                    {
                      accountId: disabledAccount.id,
                      label: "Archive",
                      purchasedAt: "2026-09-21T14:10:00Z",
                    },
                  ]
                : []),
            ],
      download: {
        status:
          i < 2
            ? "downloading"
            : i === 6
              ? "failed"
              : i >= 8 && i <= 11
                ? "downloaded"
                : "notDownloaded",
        localPath:
          i >= 8 && i <= 11
            ? `/Users/example/Library/RJ${15000000 + i}/Collection`
            : null,
        stagingPath:
          i < 2 || i === 6
            ? `/Users/example/Downloads/RJ${15000000 + i}.part`
            : null,
        unpackPolicy: "unpackWhenRecognized",
        bytesReceived:
          i < 2
            ? i === 0
              ? 322_000_000
              : 482_000_000
            : i === 6
              ? 176_000_000
              : i >= 8 && i <= 11
                ? 482_000_000
                : 0,
        bytesTotal: 482_000_000,
        errorCode: i === 6 ? "network" : null,
        errorMessage:
          i === 6
            ? "The download stream ended before the archive was complete."
            : null,
        startedAt: null,
        completedAt: null,
        updatedAt: null,
      },
    }),
  );
  const jobs: JobSnapshot[] =
    scenario === "empty"
      ? []
      : [
          ...Array.from({ length: 2 }, (_, i) => ({
            ...runningDownloadJob,
            id: `running-${i}`,
            title: titles[i],
            phase: i === 0 ? "downloading" : "unpacking",
            metadata: { workId: `RJ${15000000 + i}` },
            progress:
              i === 0
                ? { current: 322_000_000, total: 482_000_000, unit: "bytes" }
                : { current: 18, total: 100, unit: "files" },
          })),
          ...Array.from({ length: scenario === "long" ? 18 : 4 }, (_, i) => ({
            ...queuedDownloadJob,
            id: `queued-${i}`,
            title: titles[(i + 2) % titles.length],
            metadata: { workId: `RJ${15000002 + i}` },
          })),
          { ...accountSyncJob, title: "Sync Secondary purchases" },
          {
            ...failedDownloadJob,
            title: titles[6],
            metadata: { workId: "RJ15000006" },
          },
          ...Array.from({ length: scenario === "long" ? 35 : 3 }, (_, i) => ({
            ...runningDownloadJob,
            id: `finished-${i}`,
            title: titles[(i + 8) % titles.length],
            metadata: { workId: `RJ${15000008 + i}` },
            status: "succeeded" as const,
            cancellable: false,
            phase: "completed",
            progress: {
              current: 482_000_000,
              total: 482_000_000,
              unit: "bytes",
            },
            finishedAt: "2026-09-28T05:10:00Z",
          })),
        ];
  const events: AuditEvent[] = Array.from(
    { length: scenario === "empty" ? 0 : scenario === "long" ? 100 : 24 },
    (_, i) => ({
      at: `2026-09-28T05:${String(59 - (i % 60)).padStart(2, "0")}:00Z`,
      level: i % 7 === 0 ? "error" : "info",
      operation: i % 7 === 0 ? "work.download" : "account.sync",
      outcome: i % 7 === 0 ? "failed" : "succeeded",
      message:
        i % 7 === 0 ? "Download interrupted" : "Library metadata updated",
      errorCode: i % 7 === 0 ? "network" : null,
      errorMessage:
        i % 7 === 0
          ? "The download stream ended before the archive was complete."
          : null,
      details: {
        jobId: i % 7 === 0 ? failedDownloadJob.id : accountSyncJob.id,
      },
      operationId: `op-preview-${i}`,
    }),
  );
  return {
    products,
    jobs,
    events,
    accounts:
      scenario === "empty"
        ? []
        : [primaryAccount, syncingAccount, disabledAccount],
  };
}

export function isActive(job: JobSnapshot) {
  return ["queued", "running", "cancelling"].includes(job.status);
}
export function progress(job: JobSnapshot) {
  return downloadQueueProgressPercent(job) ?? 0;
}
export function jobDetail(job: JobSnapshot) {
  if (job.error) return job.error.message;
  if (job.status === "queued") return "Waiting to start";
  if (job.status === "succeeded") return "Saved to your library";
  if (job.status === "cancelled") return "Cancelled";
  if (job.kind === "accountSync") return "42 of 120 works loaded";
  if (job.phase === "unpacking")
    return `${job.progress?.current ?? 0} / ${job.progress?.total ?? "?"} files unpacked`;
  return `${Math.round((job.progress?.current ?? 0) / 1e6)} / ${Math.round((job.progress?.total ?? 0) / 1e6)} MB`;
}
