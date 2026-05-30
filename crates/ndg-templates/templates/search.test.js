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
