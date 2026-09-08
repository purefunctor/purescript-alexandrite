export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export function nested(first) {
  return (second) => first;
}
export function caseBody(maybe) {
  if (maybe === "Nothing") {
    return 0 | 0;
  }
  if (maybe.tag === "Just") {
    const { _1: value } = maybe;
    return value;
  }
  throw new Error("Pattern match failure");
}
