import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    headers: {
      "Cache-Control": "no-store",
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  resolve: {
    alias: {
      "@nest/components": path.resolve(__dirname, "../../../../core/crates/nest-react-components/src"),
    },
    dedupe: ["clsx", "tailwind-merge", "lucide-react", "react", "react-dom"],
  },
  optimizeDeps: {
    include: ["lucide-react", "clsx", "tailwind-merge"],
  },
});
