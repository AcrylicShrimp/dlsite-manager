import type { Meta, StoryObj } from "@storybook/sveltekit";
import ProductionAppStory from "./harnesses/ProductionAppStory.svelte";
const meta = {
  title: "App/Production workspace",
  component: ProductionAppStory,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof ProductionAppStory>;
export default meta;
export const Interactive: StoryObj<typeof meta> = {};
