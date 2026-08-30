import source from "./main.js" with { type: "text" };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function loadMain(requestAnimationFrame, browser = {}) {
  const window = {
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
    `${source}; return { loadClientPage, scrollToOption, transitionClientPage };`,
  )(window, document, requestAnimationFrame, browser.fetch, browser.DOMParser);
}

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
