export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function fromMaybe($a) {
  return ($maybe) => {
    if ($maybe.tag === "Just") {
      const fallback = $a;
      const whole = $maybe;
      const { _1: value } = $maybe;
      return value;
    }
    if ($maybe === "Nothing") {
      const fallback$1 = $a;
      return fallback$1;
    }
    throw new Error("Pattern match failure");
  };
}
