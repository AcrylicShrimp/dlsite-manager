import type { Meta, StoryObj } from "@storybook/sveltekit";
import DownloadsViewStory from "./harnesses/DownloadsViewStory.svelte";

const meta = {
  title: "Downloads/View",
  component: DownloadsViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "active" },
} satisfies Meta<typeof DownloadsViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ActiveQueue: Story = {};
export const Loading: Story = { args: { viewState: "loading" } };
export const Empty: Story = { args: { viewState: "empty" } };
