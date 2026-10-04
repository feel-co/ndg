import source from "./main.js" with { type: "text" };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function loadMain(requestAnimationFrame, browser = {}) {
  const window = browser.window ?? {
    requestIdleCallback() {},
    cancelIdleCallback() {},
  };
  const document = browser.document ?? { addEventListener() {} };
  return new Function(
    "window",
    "document",
    "requestAnimationFrame",
    "fetch",
    "DOMParser",
    "IntersectionObserver",
    `${source}; return { loadClientPage, scrollToOption, transitionClientPage, setupOptionChunkLoading, syncThemedImages, nextTheme };`,
  )(
    window,
    document,
    requestAnimationFrame,
    browser.fetch,
    browser.DOMParser,
    browser.IntersectionObserver,
  );
}

Deno.test("scrolling loads one option chunk at a time", async () => {
  const requested = [];
  let observe;
  class IntersectionObserver {
    constructor(callback) {
      observe = callback;
    }
    observe() {}
    disconnect() {}
  }
  const chunks = [0, 1].map((index) => ({
    dataset: { src: `chunk-${index}.html` },
    removeAttribute() {},
    set innerHTML(_html) {},
  }));
  const status = { textContent: "", remove() {} };
  const loader = {
    querySelector: () => status,
    querySelectorAll: () => chunks,
  };
  const document = {
    addEventListener() {},
    getElementById: () => ({
      textContent: JSON.stringify({ option_chunks: {} }),
    }),
    querySelector: () => loader,
  };
  const window = {
    IntersectionObserver,
    location: { hash: "" },
    addEventListener() {},
    requestIdleCallback() {},
    cancelIdleCallback() {},
  };
  const fetch = (url) => {
    requested.push(url);
    return Promise.resolve({ ok: true, text: () => Promise.resolve(url) });
  };
  const { setupOptionChunkLoading } = loadMain(() => {}, {
    document,
    fetch,
    IntersectionObserver,
    window,
  });

  setupOptionChunkLoading(new AbortController().signal);
  observe([{ isIntersecting: true }]);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert(requested.join(",") === "chunk-0.html", "only the next chunk loads");

  observe([{ isIntersecting: true }]);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert(
    requested.join(",") === "chunk-0.html,chunk-1.html",
    "the following intersection loads the following chunk",
  );
});

Deno.test("deep option anchors load every preceding chunk", async () => {
  const requested = [];
  let targetAvailable = false;
  const targetClasses = new Set();
  const target = {
    isConnected: true,
    classList: {
      add: (name) => targetClasses.add(name),
      contains: (name) => name === "option",
    },
    closest: () => ({ classList: { add() {}, remove() {} } }),
    scrollIntoView() {},
  };
  const chunks = [0, 1].map((index) => ({
    dataset: { src: `chunk-${index}.html` },
    classList: { add() {} },
    removeAttribute() {},
    set innerHTML(_html) {
      if (index === 1) targetAvailable = true;
    },
  }));
  const status = { textContent: "", remove() {} };
  const loader = {
    querySelector: () => status,
    querySelectorAll: () => chunks,
  };
  const manifest = {
    textContent: JSON.stringify({ option_chunks: { "option-deep": 1 } }),
  };
  const document = {
    addEventListener() {},
    getElementById(id) {
      if (id === "options-chunk-manifest") return manifest;
      return id === "option-deep" && targetAvailable ? target : null;
    },
    querySelector: () => loader,
  };
  const window = {
    location: { hash: "#option-deep" },
    addEventListener() {},
    requestIdleCallback() {},
    cancelIdleCallback() {},
  };
  const fetch = (url) => {
    requested.push(url);
    return Promise.resolve({ ok: true, text: () => Promise.resolve(url) });
  };
  const { setupOptionChunkLoading } = loadMain(() => {}, {
    document,
    fetch,
    window,
  });

  const loading = setupOptionChunkLoading(new AbortController().signal);
  await loading.hashReady;

  assert(
    requested.join(",") === "chunk-0.html,chunk-1.html",
    "a deep anchor must load every preceding chunk to keep its position stable",
  );
  assert(targetClasses.has("highlight"), "the deep option must be highlighted");
});

Deno.test("client navigation parses only a complete response", async () => {
  let finishResponse;
  let parsedMarkup;
  const fetch = () =>
    Promise.resolve({
      ok: true,
      text: () =>
        new Promise((resolve) => {
          finishResponse = resolve;
        }),
    });
  class DOMParser {
    parseFromString(markup) {
      parsedMarkup = markup;
      return { markup };
    }
  }
  const { loadClientPage } = loadMain(() => {}, { fetch, DOMParser });

  const loading = loadClientPage("options.html");
  await Promise.resolve();
  await Promise.resolve();
  assert(parsedMarkup === undefined, "partial markup must not be parsed");

  finishResponse("<html>complete</html>");
  const page = await loading;
  assert(
    parsedMarkup === "<html>complete</html>",
    "complete markup must be parsed",
  );
  assert(page.markup === parsedMarkup, "the parsed page must be returned");
});

Deno.test(
  "client navigation keeps the old view until replacement",
  async () => {
    let finishTransition;
    let replaced = false;
    const document = {
      addEventListener() {},
      startViewTransition(callback) {
        return {
          updateCallbackDone: new Promise((resolve) => {
            finishTransition = () => {
              callback();
              resolve();
            };
          }),
        };
      },
    };
    const { transitionClientPage } = loadMain(() => {}, { document });

    const transitioned = transitionClientPage(() => {
      replaced = true;
    });
    assert(!replaced, "replacement must wait for the captured old view");
    finishTransition();
    await transitioned;
    assert(replaced, "replacement must run inside the transition callback");
  },
);

Deno.test("option scrolling reveals contained cards before aligning", () => {
  const frames = [];
  const { scrollToOption } = loadMain((callback) => frames.push(callback));
  const classes = new Set();
  const calls = [];
  const container = {
    classList: {
      add: (name) => classes.add(name),
      remove: (name) => classes.delete(name),
    },
  };
  const option = {
    isConnected: true,
    closest: () => container,
    scrollIntoView: (settings) => calls.push(settings),
  };

  scrollToOption(option);
  assert(classes.has("options-revealed"), "options must be revealed first");
  assert(calls.length === 0, "scrolling must wait for layout");

  frames.shift()();
  assert(calls.length === 0, "scrolling must wait for a second layout frame");
  frames.shift()();
  assert(calls.length === 1, "the option must be aligned once");
  assert(calls[0].behavior === "instant", "deep jumps must not animate");
  assert(calls[0].block === "start", "the option must align to the top");
  assert(
    classes.has("options-revealed"),
    "options must stay visible while aligning",
  );

  frames.shift()();
  assert(!classes.has("options-revealed"), "containment must resume afterward");
});

function themeBrowser(dark) {
  return {
    window: {
      requestIdleCallback() {},
      cancelIdleCallback() {},
      matchMedia: () => ({ matches: dark }),
    },
    document: { addEventListener() {} },
  };
}

Deno.test("a selected theme selects the matching picture source", () => {
  const { syncThemedImages } = loadMain(() => {}, themeBrowser(false));
  const dark = { dataset: {}, media: "(prefers-color-scheme: dark)" };
  const wide = {
    dataset: {},
    media: "(min-width: 800px) and (prefers-color-scheme: light)",
  };
  const root = { querySelectorAll: () => [dark, wide] };

  syncThemedImages("dark", root);
  assert(dark.media === "(grid: 0)", "the dark source always matches");
  assert(
    wide.media === "(min-width: 800px) and (grid: 1)",
    "the light source never matches, and keeps its other conditions",
  );

  syncThemedImages("light", root);
  assert(dark.media === "(grid: 1)", "the dark source never matches");
  assert(
    wide.media === "(min-width: 800px) and (grid: 0)",
    "the light source matches on a wide screen",
  );

  syncThemedImages("auto", root);
  assert(
    dark.media === "(prefers-color-scheme: dark)",
    "the original query is restored",
  );
  assert(
    wide.media === "(min-width: 800px) and (prefers-color-scheme: light)",
    "the original query is restored",
  );
});

Deno.test("the theme toggle goes back to the system theme", () => {
  const { nextTheme } = loadMain(() => {}, themeBrowser(false));
  assert(nextTheme("auto", "light") === "dark", "auto in light selects dark");
  assert(nextTheme("dark", "light") === "auto", "dark goes back to auto");
  assert(nextTheme("auto", "dark") === "light", "auto in dark selects light");
  assert(nextTheme("light", "light") === "dark", "a pinned light selects dark");
  assert(nextTheme("light", "dark") === "auto", "light goes back to auto");
});
