import source from "./main.js" with { type: "text" };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function loadMain(requestAnimationFrame) {
  const window = {
    requestIdleCallback() {},
    cancelIdleCallback() {},
  };
  const document = { addEventListener() {} };
  return new Function(
    "window",
    "document",
    "requestAnimationFrame",
    `${source}; return { scrollToOption };`,
  )(window, document, requestAnimationFrame);
}

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
