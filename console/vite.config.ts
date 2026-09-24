import { defineConfig } from "vite";
export default defineConfig({
  root: "console",
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    host: "127.0.0.1",
    port: 5173,
    proxy: {
      "/api/gen2": "http://127.0.0.1:7332",
      "/api": "http://127.0.0.1:7331",
    },
  },
});
