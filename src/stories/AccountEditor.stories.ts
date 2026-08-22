import type { Meta, StoryObj } from "@storybook/sveltekit";
import AccountEditorStory from "./harnesses/AccountEditorStory.svelte";

const meta = {
  title: "Accounts/Editor",
  component: AccountEditorStory,
  parameters: { layout: "fullscreen" },
  args: { editorState: "new" },
} satisfies Meta<typeof AccountEditorStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const New: Story = {};
export const Editing: Story = { args: { editorState: "editing" } };
export const Saving: Story = { args: { editorState: "saving" } };
