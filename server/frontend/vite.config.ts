// SPDX-License-Identifier: MPL-2.0
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const backend = "http://127.0.0.1:8080";
/** The values the server fills into `index.html`: meta tag names and their placeholders. */
const filled = [
  ["entrylens-title", "%ENTRYLENS_TITLE%"],
  ["entrylens-version", "%ENTRYLENS_VERSION%"],
];

/**
 * In development, fills in the site's name and the server's version like the server does, taken
 * from the page the running backend serves.
 */
function serverValues(): Plugin {
  return {
    name: "entrylens-values",
    apply: "serve",
    async transformIndexHtml(html) {
      try {
        const page = await (await fetch(`${backend}/`)).text();
        for (const [name, placeholder] of filled) {
          const value = new RegExp(
            `<meta name="${name}" content="([^"]*)"`,
          ).exec(page)?.[1];
          if (value && value !== placeholder)
            html = html.replaceAll(placeholder, value);
        }
        return html;
      } catch {
        return html;
      }
    },
  };
}

export default defineConfig({
  plugins: [react(), serverValues()],
  // Served from the LAN and cached by browsers; one bundle is fine.
  build: { chunkSizeWarningLimit: 2000 },
  server: {
    // Reachable from the LAN; it forwards the API to the backend on this machine.
    host: true,
    proxy: { "/api": backend, "/livez": backend, "/readyz": backend },
  },
});
