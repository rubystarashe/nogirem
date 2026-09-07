import { svelte } from "@sveltejs/vite-plugin-svelte"
import { defineConfig } from "vite"
import { resolve } from "node:path"

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  build: {
    outDir: "dist",
    emptyOutDir: false,
    rollupOptions: {
      input: {
        main: resolve("index.html"),
        characterGuide: resolve("character-guide.html"),
        dxvkManager: resolve("dxvk-manager.html"),
        dxvkGuide: resolve("dxvk-guide.html"),
        blackboxManager: resolve("blackbox-manager.html"),
        blackboxEditor: resolve("blackbox-editor.html"),
      },
    },
  },
})
