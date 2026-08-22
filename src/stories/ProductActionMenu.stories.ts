import type { Meta, StoryObj } from "@storybook/sveltekit";
import ProductActionMenuStory from "./harnesses/ProductActionMenuStory.svelte";

const meta = {
  title: "Library/Product Action Menu",
  component: ProductActionMenuStory,
  parameters: { layout: "fullscreen" },
  args: { downloadStatus: "notDownloaded", busy: false },
} satisfies Meta<typeof ProductActionMenuStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NotDownloaded: Story = {};

export const Downloaded: Story = {
  args: { downloadStatus: "downloaded" },
};

export const FailedDownload: Story = {
  args: { downloadStatus: "failed" },
};

export const Busy: Story = {
  args: { downloadStatus: "downloaded", busy: true },
};
