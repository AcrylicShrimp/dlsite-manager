import type { Meta, StoryObj } from "@storybook/sveltekit";
import ProductImagePreviewStory from "./harnesses/ProductImagePreviewStory.svelte";

const meta = {
  title: "Library/Product Image Preview",
  component: ProductImagePreviewStory,
  parameters: { layout: "fullscreen" },
  args: { longTitle: false },
} satisfies Meta<typeof ProductImagePreviewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const LongTitle: Story = {
  args: { longTitle: true },
};
