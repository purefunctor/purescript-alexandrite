export const contravariantEmptyType = /* @__PURE__ */ (() => {
  const $closure = ($function) => {
    return (value) => {
      throw new Error("Pattern match failure");
    };
  };
  return { cmap: $closure };
})();
