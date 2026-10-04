/*
 * Draws Mermaid diagrams. When `mermaid.enable` is set, ndg renders each
 * `mermaid` fenced code block as a `<pre class="mermaid">` element and adds
 * this script to each page. The `data-mermaid-src` attribute of the script
 * element gives the location of the Mermaid library.
 *
 * The script loads the library only when the page has a diagram, because the
 * library is large. It also draws the diagrams that the page adds later, for
 * example when the options page loads more options.
 *
 * The diagrams use the theme of `<html data-theme>`, or the color scheme of the
 * system when the page has no `data-theme`. On an `ndg:themechange` event, the
 * script draws the diagrams again in the new theme.
 */
(() => {
  const script = document.currentScript;
  // Resolve the location now: client-side navigation can change the page URL.
  const source = new URL(script.dataset.mermaidSrc, document.baseURI).href;
  const selector = "pre.mermaid:not([data-processed])";
  // The text of each diagram, because Mermaid replaces it with the drawing.
  const sources = new WeakMap();
  let library;
  let queue = Promise.resolve();

  function initialize(mermaid) {
    const theme = document.documentElement.dataset.theme;
    const dark = theme
      ? theme === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: dark ? "dark" : "default",
    });
  }

  function loadLibrary() {
    library ??= new Promise((resolve, reject) => {
      const element = document.createElement("script");
      element.src = source;
      element.addEventListener("load", () => {
        initialize(globalThis.mermaid);
        resolve(globalThis.mermaid);
      });
      element.addEventListener("error", () =>
        reject(new Error(`Failed to load Mermaid from ${source}`)),
      );
      document.head.append(element);
    });
    return library;
  }

  function draw() {
    if (!document.querySelector(selector)) return;
    // Draw one batch at a time, so that no diagram is drawn twice.
    queue = queue
      .then(async () => {
        const mermaid = await loadLibrary();
        for (const element of document.querySelectorAll(selector)) {
          if (!sources.has(element)) sources.set(element, element.textContent);
        }
        await mermaid.run({ querySelector: selector });
      })
      .catch((error) => console.error(error));
  }

  function redraw() {
    // Without the library, no diagram is drawn yet.
    if (!library) return;
    queue = queue
      .then(async () => {
        const mermaid = await library;
        initialize(mermaid);
        for (const element of document.querySelectorAll(
          "pre.mermaid[data-processed]",
        )) {
          if (!sources.has(element)) continue;
          element.textContent = sources.get(element);
          element.removeAttribute("data-processed");
        }
        await mermaid.run({ querySelector: selector });
      })
      .catch((error) => console.error(error));
  }

  draw();
  document.addEventListener("ndg:themechange", redraw);
  new MutationObserver(draw).observe(document.body, {
    childList: true,
    subtree: true,
  });
})();
