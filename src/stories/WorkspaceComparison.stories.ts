import type { Meta, StoryObj } from "@storybook/sveltekit";
import WorkspaceComparison from "./redesign/WorkspaceComparison.svelte";
const meta = {
  title: "Redesign/Compare",
  component: WorkspaceComparison,
  parameters: { layout: "fullscreen", controls: { disable: true } },
} satisfies Meta<typeof WorkspaceComparison>;
export default meta;
type Story = StoryObj<typeof meta>;
export const SideBySide: Story = {};
