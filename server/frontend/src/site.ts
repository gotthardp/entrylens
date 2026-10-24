// SPDX-License-Identifier: MPL-2.0

/** Name of the site, filled into `index.html` by the server from its configuration. */
export const siteTitle = (() => {
  const title = document
    .querySelector('meta[name="entrylens-title"]')
    ?.getAttribute("content");
  return title && title !== "%ENTRYLENS_TITLE%" ? title : "Entrylens";
})();

document.title = siteTitle;

/** Version of the server, filled into `index.html` by the server; empty if unknown. */
export const serverVersion = (() => {
  const version = document
    .querySelector('meta[name="entrylens-version"]')
    ?.getAttribute("content");
  return version && version !== "%ENTRYLENS_VERSION%" ? version : "";
})();
