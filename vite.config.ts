import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [solid()],
  // relativní cesty — absolutní dávají v produkčním buildu Tauri bílou obrazovku
  base: "./",
  // tři stránky: šuplík (index.html), okno lístku na plochu (pin.html → src/pin/main.tsx, jen jeho kód)
  // a okno widgetu na ploše (desk.html → src/desk/main.tsx); Ledge Studio = Nastavení v okně (settings.html →
  // src/studio/main.tsx, kolo 24)
  build: { target: "chrome120", rolldownOptions: { input: { index: "index.html", pin: "pin.html", desk: "desk.html", settings: "settings.html" } } },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
