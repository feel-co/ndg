/*
 * Draws Mermaid diagrams. When `mermaid.enable` is set, ndg renders each
 * `mermaid` fenced code block as a `<pre class="mermaid">` element and adds
 * this script to each page. The `data-mermaid-src` attribute of the script
 * element gives the location of the Mermaid library.
 *
 * The script loads the library only when the page has a diagram, because the
 * library is large. It also draws the diagrams that the page adds later, for
 * example when the options page loads more options.
 */
(() => {
  const script = document.currentScript;
  // Resolve the location now: client-side navigation can change the page URL.
  const source = new URL(script.dataset.mermaidSrc, document.baseURI).href;
  const selector = "pre.mermaid:not([data-processed])";
  let library;
  let queue = Promise.resolve();

  function loadLibrary() {
    library ??= new Promise((resolve, reject) => {
      const element = document.createElement("script");
      element.src = source;
      element.addEventListener("load", () => {
        const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        globalThis.mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: dark ? "dark" : "default",
        });
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
        await mermaid.run({ querySelector: selector });
      })
      .catch((error) => console.error(error));
  }

  draw();
  new MutationObserver(draw).observe(document.body, {
    childList: true,
    subtree: true,
  });
})();
