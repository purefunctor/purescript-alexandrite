export const second = (() => {
  throw new Error("Top-level value initializer cycle");
})();
export const first = (() => {
  throw new Error("Top-level value initializer cycle");
})();
