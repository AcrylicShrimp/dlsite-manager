import type { Meta, StoryObj } from "@storybook/sveltekit";
import FieldStory from "./harnesses/FieldStory.svelte";

const meta = {
  title: "Foundations/Field",
  component: FieldStory,
  tags: ["autodocs"],
  args: {
    label: "Library folder",
    help: "Downloaded works are finalized under this folder.",
    placeholder: "/Users/example/Library",
    disabled: false,
    type: "text",
  },
  argTypes: {
    type: { control: "select", options: ["text", "search", "password"] },
  },
} satisfies Meta<typeof FieldStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Search: Story = {
  args: {
    label: "Search library",
    help: "Title, maker, work ID, or custom tag",
    placeholder: "Search works",
    type: "search",
  },
};

export const Disabled: Story = {
  args: {
    label: "Download staging folder",
    help: "Unavailable while a download is being finalized.",
    disabled: true,
  },
};
