import type { Meta, StoryObj } from "@storybook/sveltekit";
import SettingsViewStory from "./harnesses/SettingsViewStory.svelte";

const meta = {
  title: "Settings/View",
  component: SettingsViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "ready" },
} satisfies Meta<typeof SettingsViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {};
export const Loading: Story = { args: { viewState: "loading" } };
export const Saving: Story = { args: { viewState: "saving" } };
export const CheckingForUpdates: Story = { args: { viewState: "checking" } };
export const DownloadingUpdate: Story = { args: { viewState: "downloading" } };
export const InstallingUpdate: Story = { args: { viewState: "installing" } };
