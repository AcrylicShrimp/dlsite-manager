import type { Meta, StoryObj } from "@storybook/sveltekit";
import type { ProductDetail } from "$lib/model/types";
import ProductDetailDialogStory from "./harnesses/ProductDetailDialogStory.svelte";

const thumbnail =
  "data:image/svg+xml," +
  encodeURIComponent(`
    <svg xmlns="http://www.w3.org/2000/svg" width="480" height="480" viewBox="0 0 480 480">
      <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#293d34"/><stop offset="1" stop-color="#101416"/></linearGradient></defs>
      <rect width="480" height="480" fill="url(#g)"/>
      <circle cx="240" cy="190" r="104" fill="#95c29b" fill-opacity=".14"/>
      <path d="M136 304c42-82 166-82 208 0" fill="none" stroke="#95c29b" stroke-width="18" stroke-linecap="round"/>
      <text x="240" y="402" text-anchor="middle" fill="#edf2f6" font-family="system-ui" font-size="35" font-weight="700">RJ SAMPLE</text>
    </svg>
  `);

const downloadedDetail: ProductDetail = {
  workId: "RJ01553954",
  title: "A Long Evening at the Observatory — Binaural Voice Drama",
  titleVariants: [
    { language: "en_US", value: "A Long Evening at the Observatory" },
    { language: "ja_JP", value: "天文台で過ごす長い夜" },
  ],
  makerId: "RG01234567",
  makerName: "North Window Studio",
  makerNames: [{ language: "en_US", value: "North Window Studio" }],
  workType: "SOU",
  ageCategory: "all",
  thumbnailUrl: thumbnail,
  contentSizeBytes: 482_000_000,
  registeredAt: "2026-05-04T00:00:00Z",
  publishedAt: "2026-05-05T00:00:00Z",
  updatedAt: "2026-05-18T00:00:00Z",
  lastDetailSyncAt: "2026-08-12T07:30:00Z",
  earliestPurchasedAt: "2026-05-20T08:30:00Z",
  latestPurchasedAt: "2026-05-21T14:10:00Z",
  creditGroups: [
    { kind: "voice", label: "Voice", names: ["Akari Example"] },
    { kind: "scenario", label: "Scenario", names: ["M. Hoshino"] },
    { kind: "illust", label: "Illustration", names: ["Studio K"] },
    { kind: "music", label: "Music", names: ["Night Signal"] },
  ],
  tags: [],
  customTags: [{ name: "Favorites" }, { name: "Sleep" }, { name: "Long-form listening" }],
  download: {
    status: "downloaded",
    localPath: "/Users/example/Library/RJ01553954/A Long Evening at the Observatory",
    stagingPath: null,
    unpackPolicy: "unpackWhenRecognized",
    bytesReceived: 482_000_000,
    bytesTotal: 482_000_000,
    errorCode: null,
    errorMessage: null,
    startedAt: "2026-05-21T14:12:00Z",
    completedAt: "2026-05-21T14:15:00Z",
    updatedAt: "2026-05-21T14:15:00Z",
  },
  owners: [
    { accountId: "primary", label: "Primary", purchasedAt: "2026-05-20T08:30:00Z" },
    { accountId: "archive", label: "Archive", purchasedAt: "2026-05-21T14:10:00Z" },
  ],
};

const localOnlyDetail: ProductDetail = {
  ...downloadedDetail,
  workId: "RJ01234567",
  title: "Imported local work with partially available metadata and a deliberately long title",
  titleVariants: [],
  makerId: null,
  makerName: null,
  makerNames: [],
  workType: "COM",
  ageCategory: "r18",
  thumbnailUrl: null,
  contentSizeBytes: null,
  registeredAt: null,
  publishedAt: null,
  updatedAt: null,
  earliestPurchasedAt: null,
  latestPurchasedAt: null,
  creditGroups: [],
  customTags: [],
  download: {
    ...downloadedDetail.download,
    localPath: "/Users/example/Library/RJ01234567",
  },
  owners: [{ accountId: "__local__", label: "Local", purchasedAt: null }],
};

const meta = {
  title: "Library/Product Detail Dialog",
  component: ProductDetailDialogStory,
  parameters: { layout: "fullscreen" },
  args: { detail: downloadedDetail },
} satisfies Meta<typeof ProductDetailDialogStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DownloadedOwned: Story = {};

export const LocalOnlySparse: Story = {
  args: { detail: localOnlyDetail },
};

export const FailedDownload: Story = {
  args: {
    detail: {
      ...downloadedDetail,
      download: {
        ...downloadedDetail.download,
        status: "failed",
        localPath: null,
        errorCode: "network",
        errorMessage: "The download stream ended before the expected archive size was reached.",
      },
    },
  },
};
