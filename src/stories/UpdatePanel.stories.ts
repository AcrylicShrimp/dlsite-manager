import type { Meta, StoryObj } from "@storybook/sveltekit";
import UpdatePanelStory from "./harnesses/UpdatePanelStory.svelte";

const meta = {
  title: "Settings/Update Panel",
  component: UpdatePanelStory,
  parameters: { layout: "fullscreen" },
  args: { phase: "idle", message: "" },
} satisfies Meta<typeof UpdatePanelStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {};
export const Checking: Story = { args: { phase: "checking", message: "Checking for updates" } };
export const Downloading: Story = { args: { phase: "downloading", message: "Downloading 3.2.3 68%" } };
export const Installing: Story = { args: { phase: "installing", message: "Installing 3.2.3" } };
