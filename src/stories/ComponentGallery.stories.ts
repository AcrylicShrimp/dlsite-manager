import type { Meta, StoryObj } from "@storybook/sveltekit";
import ComponentGallery from "./ComponentGallery.svelte";

const meta = {
  title: "Foundations/Overview",
  component: ComponentGallery,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof ComponentGallery>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AllFoundations: Story = {};
