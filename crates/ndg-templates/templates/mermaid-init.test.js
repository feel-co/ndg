import source from "./mermaid-init.js" with { type: "text" };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function loadMermaidInit({ diagrams, dark = false }) {
  const appended = [];
  const runs = [];
  const initialized = [];
  let observe;
  const mermaid = {
    initialize(config) {
      initialized.push(config);
    },
    run({ querySelector }) {
      runs.push(querySelector);
      diagrams.processed = true;
      return Promise.resolve();
    },
  };
  const globals = {};
  const document = {
    baseURI: "https://example.com/docs/page.html",
    currentScript: { dataset: { mermaidSrc: "../assets/mermaid.min.js" } },
    body: {},
    head: {
      append(element) {
        appended.push(element);
        globals.mermaid = mermaid;
        element.listeners.load();
      },
    },
    createElement: () => ({
      listeners: {},
      addEventListener(event, listener) {
        this.listeners[event] = listener;
      },
    }),
    querySelector: () =>
      diagrams.count > 0 && !diagrams.processed ? {} : null,
  };
  const window = { matchMedia: () => ({ matches: dark }) };
  class MutationObserver {
    constructor(callback) {
      observe = callback;
    }
    observe() {}
  }
  new Function(
    "document",
    "window",
    "MutationObserver",
    "globalThis",
    source,
  )(document, window, MutationObserver, globals);
  return { appended, runs, initialized, mutate: () => observe() };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

Deno.test("a page without diagrams does not load Mermaid", async () => {
  const page = loadMermaidInit({ diagrams: { count: 0 } });
  await settle();
  assert(page.appended.length === 0, "the library is not loaded");
});

Deno.test("a page with diagrams loads Mermaid once and draws", async () => {
  const diagrams = { count: 1 };
  const page = loadMermaidInit({ diagrams, dark: true });
  await settle();
  assert(page.appended.length === 1, "the library is loaded once");
  assert(
    page.appended[0].src === "https://example.com/assets/mermaid.min.js",
    "the library location is resolved against the page",
  );
  assert(page.initialized[0].theme === "dark", "the dark theme is selected");
  assert(page.initialized[0].startOnLoad === false, "Mermaid does not start");
  assert(page.runs.length === 1, "the diagrams are drawn");

  diagrams.processed = false;
  page.mutate();
  await settle();
  assert(page.appended.length === 1, "the library is not loaded again");
  assert(page.runs.length === 2, "new diagrams are drawn");
});
