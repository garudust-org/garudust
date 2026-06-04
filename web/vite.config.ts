import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Build output lands in `dist/`, which the gateway embeds via rust-embed
// (feature `web-ui`). In dev, proxy the gateway so the same relative URLs
// (`/api`, `/chat/ws`, `/health`) work in both browser and the embedded build.
const GATEWAY = "http://localhost:3000";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    proxy: {
      "/api": GATEWAY,
      "/health": GATEWAY,
      "/metrics": GATEWAY,
      // ws: true upgrades /chat/ws through the dev proxy to the gateway.
      "/chat": { target: GATEWAY, ws: true },
    },
  },
});
