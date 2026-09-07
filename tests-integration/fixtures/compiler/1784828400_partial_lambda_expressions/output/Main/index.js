export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export function checked(partialDict) {
  const $closure = ($maybe) => {
    if ($maybe.tag === "Just") {
      const { _1: value } = $maybe;
      return value;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return $closure;
}
export function inferred(partialDict) {
  const $closure = ($maybe) => {
    if ($maybe.tag === "Just") {
      const { _1: value } = $maybe;
      return value;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return $closure;
}
