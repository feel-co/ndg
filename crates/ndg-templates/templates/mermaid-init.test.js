import source from "./mermaid-init.js" with { type: "text" };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function loadMermaidInit({ diagrams, dark = false, theme }) {
  const appended = [];
  const runs = [];
  const initialized = [];
  const listeners = {};
  let observe;
  const mermaid = {
    initialize(config) {
      initialized.push(config);
    },
    run({ querySelector }) {
      runs.push({ querySelector, text: diagrams.text });
      diagrams.text = "<svg></svg>";
      diagrams.processed = true;
      return Promise.resolve();
    },
  };
  const globals = {};
  const element = {
    get textContent() {
      return diagrams.text;
    },
    set textContent(text) {
      diagrams.text = text;
    },
    removeAttribute(name) {
      if (name === "data-processed") diagrams.processed = false;
    },
  };
  const document = {
    baseURI: "https://example.com/docs/page.html",
    currentScript: { dataset: { mermaidSrc: "../assets/mermaid.min.js" } },
    documentElement: { dataset: theme ? { theme } : {} },
    body: {},
    addEventListener(event, listener) {
      listeners[event] = listener;
    },
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
    // One element stands for all the diagrams.
    querySelectorAll(selector) {
      if (diagrams.count === 0) return [];
      if (diagrams.processed === selector.includes(":not(")) return [];
      return [element];
    },
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
  return {
    appended,
    runs,
    initialized,
    mutate: () => observe(),
    changeTheme(theme) {
      document.documentElement.dataset.theme = theme;
      listeners["ndg:themechange"]();
    },
  };
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
  diagrams.text = "graph TD; C-->D";
  page.mutate();
  await settle();
  assert(page.appended.length === 1, "the library is not loaded again");
  assert(page.runs.length === 2, "new diagrams are drawn");
});

Deno.test("a selected theme overrides the system", async () => {
  const page = loadMermaidInit({ diagrams: { count: 1 }, theme: "light" });
  await settle();
  assert(page.initialized[0].theme === "default", "the light theme is used");
});

Deno.test("a theme change draws the diagrams again", async () => {
  const diagrams = { count: 1, text: "graph TD; A-->B" };
  const page = loadMermaidInit({ diagrams });
  await settle();
  assert(page.initialized[0].theme === "default", "the light theme is used");

  page.changeTheme("dark");
  await settle();
  assert(page.initialized[1].theme === "dark", "the dark theme is selected");
  assert(page.runs.length === 2, "the diagrams are drawn again");
  assert(
    page.runs[1].text === "graph TD; A-->B",
    "the diagrams are drawn from their text, not the old drawing",
  );
});

Deno.test("a theme change before the first drawing does nothing", async () => {
  const page = loadMermaidInit({ diagrams: { count: 0 } });
  page.changeTheme("dark");
  await settle();
  assert(page.appended.length === 0, "the library is not loaded");
});
