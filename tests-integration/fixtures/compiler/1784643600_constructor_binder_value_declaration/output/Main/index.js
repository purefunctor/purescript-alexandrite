export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function fromMaybe($argument0) {
  return ($maybe) => {
    if ($maybe.tag === "Just") {
      const $default = $argument0;
      const { _1: value } = $maybe;
      return value;
    }
    if ($maybe === "Nothing") {
      const $default$1 = $argument0;
      return $default$1;
    }
    throw new Error("Pattern match failure");
  };
}
