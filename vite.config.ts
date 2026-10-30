import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    // ACKNOWLEDGEMENTS.md is large and loaded on demand by the Licenses topic.
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      input: { settings: "settings.html", overlay: "overlay.html" },
    },
  },
});
