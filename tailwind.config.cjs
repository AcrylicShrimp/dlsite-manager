/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  theme: {
    extend: {
      colors: {
        error: "#fc634c",
        ok: "#21d618",
        "0/5": "#171921",
        "1/5": "#2d3142",
        "2/5": "#4f5d75",
        "3/5": "#7a808a",
        "4/5": "#f0f6ff",
        "5/5": "#ffffff",
        point: "#ef8354",
      },
    },
  },
  plugins: [],
};
