import type { StorybookConfig } from "@storybook/sveltekit";
import tailwindcss from "@tailwindcss/vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(js|ts|svelte)"],
  addons: ["@storybook/addon-a11y"],
  framework: {
    name: "@storybook/sveltekit",
    options: {},
  },
  viteFinal: async (config) => ({
    ...config,
    plugins: [...(config.plugins ?? []), tailwindcss()],
  }),
  staticDirs: ["../static"],
};

export default config;
