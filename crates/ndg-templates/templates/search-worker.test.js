// Drive search-worker.js with a mocked worker global scope so the worker's
// own search path (not the main-thread fallback) is exercised.
const messages = [];
globalThis.self = globalThis;
globalThis.postMessage = (msg) => messages.push(msg);

await import("./search-worker.js");

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

const docs = [
  {
    id: "1",
    title: "Terminal",
    content: "Configure the terminal emulator.",
    path: "term.html",
    anchors: [],
  },
  {
    id: "2",
    title: "Sidebar",
    content: "Overview of the layout.",
    path: "sidebar.html",
    anchors: [],
  },
];

function send(message) {
  messages.length = 0;
  globalThis.onmessage({ data: message });
  return messages;
}

Deno.test("worker searches the cached index without resending documents", () => {
  send({ type: "init", data: { documents: docs, config: { minWordLength: 2 } } });
  const out = send({
    messageId: "a",
    type: "search",
    data: { query: "sidebar", limit: 8 },
  });
  assertEquals(out.length, 1, "should post exactly one results message");
  assertEquals(out[0].type, "results");
  assertEquals(out[0].messageId, "a", "messageId should be echoed back");
  assertGreater(out[0].data.length, 0, "cached docs should be searchable");
  assertEquals(out[0].data[0].doc.title, "Sidebar");
});

Deno.test("worker honours a two-character query", () => {
  // Previously the worker dropped words of length <= 2 and bailed out on
  // queries shorter than 3 chars, so "fo" returned nothing even though the
  // main thread matched it.
  send({ type: "init", data: { documents: docs, config: { minWordLength: 2 } } });
  const out = send({
    messageId: "b",
    type: "search",
    data: { query: "te", limit: 8 },
  });
  assertEquals(out[0].type, "results");
  assertGreater(out[0].data.length, 0, "two-char query should match 'Terminal'");
  assertEquals(out[0].data[0].doc.title, "Terminal");
});

Deno.test("worker keeps two-character queries below min word length searchable", () => {
  send({ type: "init", data: { documents: docs, config: { minWordLength: 3 } } });
  const out = send({
    messageId: "c",
    type: "search",
    data: { query: "te", limit: 8 },
  });
  assertEquals(out[0].type, "results");
  assertGreater(out[0].data.length, 0, "visible two-char text should match");
  assertEquals(out[0].data[0].doc.title, "Terminal");
});

Deno.test("worker returns a visible phrase made only of stopwords", () => {
  send({
    type: "init",
    data: {
      documents: [
        {
          id: "intro",
          title: "What is",
          content: "A short introduction.",
          path: "intro.html",
          anchors: [],
        },
      ],
      config: { minWordLength: 2, stopwords: ["what", "is"] },
    },
  });
  const out = send({
    messageId: "d",
    type: "search",
    data: { query: "what is", limit: 8 },
  });
  assertEquals(out[0].type, "results");
  assertEquals(out[0].data.length, 1, "stopword phrase should still match");
  assertEquals(out[0].data[0].doc.title, "What is");
});
