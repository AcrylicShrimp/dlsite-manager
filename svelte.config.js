import staticAdapter from "@sveltejs/adapter-static";
import preprocess from "svelte-preprocess";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: preprocess({
    postcss: true,
  }),

  kit: {
    adapter: staticAdapter({
      pages: "build",
      assets: "build",
      fallback: "app.html",
    }),
    alias: {
      "@app/*": "src/*",
    },
  },
};

export default config;
