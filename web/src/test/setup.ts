import "@testing-library/jest-dom/vitest";

// Node 22+ exposes a built-in `localStorage` that requires `--localstorage-file`.
// Vitest's jsdom environment sets `global.jsdom = dom` but does NOT propagate
// `localStorage` / `sessionStorage` from jsdom's window into the global scope
// (because those keys already exist on the Node.js global). Override them here
// so tests get jsdom's in-memory Storage, not Node's file-backed one.
if (typeof globalThis.jsdom !== "undefined") {
  Object.defineProperty(globalThis, "localStorage", {
    value: (globalThis as typeof globalThis & { jsdom: { window: Window } }).jsdom.window.localStorage,
    writable: true,
    configurable: true,
  });
  Object.defineProperty(globalThis, "sessionStorage", {
    value: (globalThis as typeof globalThis & { jsdom: { window: Window } }).jsdom.window.sessionStorage,
    writable: true,
    configurable: true,
  });
}
