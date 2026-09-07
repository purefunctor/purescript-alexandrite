export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function fromJust(partialDict) {
  const $closure = ($maybe) => {
    if ($maybe.tag === "Just") {
      const { _1: x } = $maybe;
      return x;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return $closure;
}
export const test = /* @__PURE__ */ (() => {
  const silent = (partialDict) => {
    const $closure = ($maybe) => {
      if ($maybe.tag === "Just") {
        const { _1: x } = $maybe;
        return x;
      } else {
        throw new Error("Pattern match failure");
      }
    };
    return $closure;
  };
  const loud = ($maybe$1) => {
    if ($maybe$1 === "Nothing") {
      return 42 | 0;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return 42 | 0;
})();
