import type { Meta, StoryObj } from "@storybook/sveltekit";
import DraftControls from "./redesign/DraftControls.svelte";

const meta = {
  title: "Redesign/Controls",
  component: DraftControls,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof DraftControls>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Shared: Story = {};
