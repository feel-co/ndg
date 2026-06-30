// Mock browser globals before loading search.js
globalThis.window = globalThis;
globalThis.window.searchNamespace = { rootPath: "" };
let workerPath = null;
globalThis.Worker = class {
  constructor(path) {
    workerPath = path;
    throw new Error("stop after capturing worker path");
  }
};
globalThis.document = {
  addEventListener: (_event, _handler) => {},
  getElementById: (_id) => null,
};

await import("./search.js");

const engine = globalThis.window.searchNamespace.engine;

Deno.test("worker path stays relative for root pages", () => {
  globalThis.window.searchNamespace.rootPath = "";
  workerPath = null;
  const warn = console.warn;
  console.warn = () => {};

  try {
    assertEquals(engine.useWebWorker, false);
    assertEquals(workerPath, "assets/search-worker.js");
  } finally {
    console.warn = warn;
  }
});

/**
 * Loads a fresh set of documents into the engine and marks it ready.
 * Each test should call this to isolate state.
 */
async function loadDocs(docs) {
  engine.config = {
    minWordLength: 2,
    stopwords: [],
    boostTitle: 100.0,
    boostContent: 30.0,
    boostAnchor: 10.0,
  };
  await engine.initializeFromDocuments(docs);
  engine.isLoaded = true;
  engine.loadError = false;
}

function assertEquals(a, b, msg) {
  if (a !== b) {
    throw new Error(
      `${msg ?? "assertEquals failed"}: expected ${JSON.stringify(b)}, got ${JSON.stringify(a)}`,
    );
  }
}

function assertGreater(a, b, msg) {
  if (!(a > b)) {
    throw new Error(
      `${msg ?? "assertGreater failed"}: expected ${JSON.stringify(a)} > ${JSON.stringify(b)}`,
    );
  }
}

// Ranking
Deno.test("title match ranks above content-only match", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Getting Started",
      content: "This page explains how to configure the sidebar navigation.",
      path: "getting-started.html",
      anchors: [],
    },
    {
      id: "2",
      title: "Sidebar",
      content: "Overview of the layout.",
      path: "sidebar.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("sidebar");
  assertGreater(results.length, 0, "should return at least one result");
  assertEquals(
    results[0].doc.title,
    "Sidebar",
    "title match should rank first",
  );
});

Deno.test("exact title match outscores partial title match", async () => {
  // Content deliberately excludes the term so scoring differences come purely
  // from the title. "Sidebar" (exact) should outscore "Sidebar Options" (partial).
  await loadDocs([
    {
      id: "1",
      title: "Sidebar Options",
      content: "Reference for all available configuration keys.",
      path: "sidebar-opts.html",
      anchors: [],
    },
    {
      id: "2",
      title: "Sidebar",
      content: "Overview of the navigation panel.",
      path: "sidebar.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("sidebar");
  assertGreater(results.length, 0, "should return results");
  assertEquals(results[0].doc.title, "Sidebar", "exact title match should rank first");
});

Deno.test("document without search term is excluded from results", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Option: hjem.users.systemd.paths",
      content: "Paths for systemd user services managed by the module.",
      path: "options.html#hjem.users.systemd.paths",
      anchors: [],
    },
    {
      id: "2",
      title: "Option: hjem.users.systemd.services",
      content: "Services for systemd user units managed by the module.",
      path: "options.html#hjem.users.systemd.services",
      anchors: [],
    },
  ]);

  const results = await engine.search("sidebar");
  assertEquals(
    results.length,
    0,
    "options unrelated to 'sidebar' should not appear in results",
  );
});

Deno.test("content-only match returns results", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Configuration",
      content: "You can enable the sidebar by setting sidebar.enable to true.",
      path: "config.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("sidebar");
  assertEquals(results.length, 1, "content match should still return results");
  assertEquals(results[0].doc.title, "Configuration");
});

Deno.test("substring search finds hyphenated identifiers", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Hooks",
      content: "The redis-test-hook validates Redis integration tests.",
      path: "hooks.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("redis");
  assertEquals(results.length, 1, "partial identifier search should return result");
  assertEquals(results[0].doc.title, "Hooks");
});

Deno.test("multi-word section heading outranks option matches", async () => {
  const optionDocs = Array.from({ length: 12 }, (_, i) => ({
    id: `option-${i}`,
    title: `Option: nvf.example.option${i}`,
    content: "This option explains what is used to configure nvf behavior.",
    path: `options.html#nvf.example.option${i}`,
    anchors: [],
  }));

  await loadDocs([
    ...optionDocs,
    {
      id: "guide",
      title: "Introduction",
      content: "What is nvf\nAn overview of the project.",
      path: "index.html",
      anchors: [
        { id: "what-is-nvf", text: "What is nvf", level: 2, tokens: [] },
      ],
    },
  ]);

  const results = await engine.search("what is", 8);
  assertGreater(results.length, 0, "should return results");
  assertEquals(results[0].doc.title, "Introduction");
  assertEquals(results[0].matchingAnchors[0].text, "What is nvf");
});

Deno.test("exact section phrase outranks partial nvf page and anchor matches", async () => {
  await loadDocs([
    {
      id: "configuring",
      title: "Configuring nvf",
      content: "DAG entries in nvf\nConfiguration details.",
      path: "configuring.html",
      anchors: [
        { id: "dag-entries", text: "DAG entries in nvf", level: 3, tokens: [] },
      ],
    },
    {
      id: "hacking",
      title: "Hacking nvf",
      content: "Developer documentation for nvf.",
      path: "hacking.html",
      anchors: [],
    },
    {
      id: "option",
      title: "Option: vim.additionalRuntimePaths",
      content: "What is used by nvf at runtime.",
      path: "options.html#vim.additionalRuntimePaths",
      anchors: [],
    },
    {
      id: "intro",
      title: "Introduction",
      content: "What is nvf\nnvf is a highly modular configuration framework.",
      path: "index.html",
      anchors: [
        { id: "what-is-nvf", text: "What is nvf", level: 3, tokens: [] },
      ],
    },
  ]);

  const results = await engine.search("What is nvf", 8);
  assertGreater(results.length, 0, "should return results");
  assertEquals(results[0].doc.title, "Introduction");
  assertEquals(results[0].matchingAnchors[0].text, "What is nvf");
});

Deno.test("section anchor id can make a page a search candidate", async () => {
  await loadDocs([
    {
      id: "other",
      title: "Configuring nvf",
      content: "Configuration details for nvf.",
      path: "configuring.html",
      anchors: [],
    },
    {
      id: "intro",
      title: "Introduction",
      content: "nvf is a Neovim framework.",
      path: "index.html",
      anchors: [
        { id: "sec-what-is-it", text: "What is nvf", level: 3, tokens: [] },
      ],
    },
  ]);

  const results = await engine.search("what is it", 8);
  assertGreater(results.length, 0, "anchor id should produce a result");
  assertEquals(results[0].doc.title, "Introduction");
  assertEquals(results[0].matchingAnchors[0].id, "sec-what-is-it");
});

Deno.test("section title ranks above option title partial match", async () => {
  await loadDocs([
    {
      id: "option",
      title: "Option: vim.nvf.enable",
      content: "Enable nvf integration.",
      path: "options.html#vim.nvf.enable",
      anchors: [],
    },
    {
      id: "intro",
      title: "Introduction",
      content: "What is nvf\nnvf is a Neovim framework.",
      path: "index.html",
      anchors: [
        { id: "sec-what-is-it", text: "What is nvf", level: 3, tokens: [] },
      ],
    },
  ]);

  const results = await engine.search("nvf", 8);
  assertGreater(results.length, 0, "should return results");
  assertEquals(results[0].doc.title, "Introduction");
  assertEquals(results[0].matchingAnchors[0].text, "What is nvf");
});

Deno.test("multi-word page title is returned by widget-sized search", async () => {
  await loadDocs([
    {
      id: "option",
      title: "Option: nvf.enable",
      content: "This option is used to enable nvf.",
      path: "options.html#nvf.enable",
      anchors: [],
    },
    {
      id: "page",
      title: "What is nvf",
      content: "An overview of the project.",
      path: "index.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("what is", 8);
  assertGreater(results.length, 0, "should return results");
  assertEquals(results[0].doc.title, "What is nvf");
});

Deno.test("options with irrelevant content don't displace title matches", async () => {
  const optionDocs = Array.from({ length: 20 }, (_, i) => ({
    id: String(i),
    title: `Option: hjem.users.systemd.paths.rule${i}`,
    content: "Configure systemd path rules for user services.",
    path: `options.html#rule${i}`,
    anchors: [],
  }));

  await loadDocs([
    ...optionDocs,
    {
      id: "page",
      title: "Sidebar",
      content: "The sidebar provides navigation links.",
      path: "sidebar.html",
      anchors: [],
    },
  ]);

  const results = await engine.search("sidebar");
  assertGreater(results.length, 0, "should return results");
  assertEquals(
    results[0].doc.title,
    "Sidebar",
    "sidebar page should rank first even with many unrelated options",
  );
});

Deno.test("empty query returns no results", async () => {
  await loadDocs([{ id: "1", title: "Test", content: "test", path: "t.html", anchors: [] }]);
  const results = await engine.search("");
  assertEquals(results.length, 0);
});

Deno.test("single-character query returns no results", async () => {
  await loadDocs([{ id: "1", title: "abc", content: "abc", path: "t.html", anchors: [] }]);
  const results = await engine.search("a");
  assertEquals(results.length, 0, "queries shorter than 2 chars should be rejected");
});

Deno.test("anchor match is returned for matching heading", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Installation",
      content: "Getting Started\nInstall the package first.\nSidebar Setup\nConfigure the sidebar.",
      path: "install.html",
      anchors: [
        { id: "getting-started", text: "Getting Started", level: 2, tokens: [] },
        { id: "sidebar-setup", text: "Sidebar Setup", level: 2, tokens: [] },
      ],
    },
  ]);

  const results = await engine.search("sidebar");
  assertGreater(results.length, 0, "should find document");
  const anchorIds = results[0].matchingAnchors.map((a) => a.id);
  assertEquals(
    anchorIds.includes("sidebar-setup"),
    true,
    "sidebar-setup anchor should be in matching anchors",
  );
});

Deno.test("two-character query still searches when min word length is higher", async () => {
  await loadDocs([
    {
      id: "1",
      title: "Terminal",
      content: "Configure the terminal emulator.",
      path: "term.html",
      anchors: [],
    },
  ]);
  engine.config = { ...engine.config, minWordLength: 3 };

  const results = await engine.search("te");
  assertEquals(results.length, 1, "visible two-character text should match");
  assertEquals(results[0].doc.title, "Terminal");
});

Deno.test("stopword-only phrase search can return visible title", async () => {
  await loadDocs([
    {
      id: "1",
      title: "What is",
      content: "A short introduction.",
      path: "intro.html",
      anchors: [],
    },
  ]);
  engine.config = { ...engine.config, stopwords: ["what", "is"] };

  const results = await engine.search("what is");
  assertEquals(results.length, 1, "visible phrase should match even as stopwords");
  assertEquals(results[0].doc.title, "What is");
});
