import type { Meta, StoryObj } from "@storybook/sveltekit";
import DownloadQueueRowStory from "./harnesses/DownloadQueueRowStory.svelte";
import { failedDownloadJob } from "./fixtures/jobs";

const meta = {
  title: "Downloads/Queue Row",
  component: DownloadQueueRowStory,
  parameters: { layout: "fullscreen" },
  args: { job: failedDownloadJob },
} satisfies Meta<typeof DownloadQueueRowStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Failed: Story = {};
