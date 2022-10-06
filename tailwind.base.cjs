const path = require("path");

/** @type {import('tailwindcss').Config} */
module.exports = {
  corePlugins: {
    preflight: false,
  },
  plugins: [require("@tailwindcss/line-clamp")],
  content: [
    path.resolve(__dirname, "packages/*/index.html"),
    path.resolve(__dirname, "packages/*/src/**/*.{vue,js,ts,jsx,tsx}"),
  ],
  theme: {
    extend: {},
  },
};
