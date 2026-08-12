import type { Meta, StoryObj } from "@storybook/sveltekit";
import BulkDownloadDialogStory from "./harnesses/BulkDownloadDialogStory.svelte";

const meta = {
  title: "Overlays/Bulk Download Dialog",
  component: BulkDownloadDialogStory,
  parameters: { layout: "fullscreen" },
  args: { kind: "confirm", failedCount: 0 },
} satisfies Meta<typeof BulkDownloadDialogStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Confirm: Story = {};

export const WithPlanningWarning: Story = {
  args: { failedCount: 2 },
};

export const NothingToDownload: Story = {
  args: { kind: "notice" },
};
