export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test($maybe) {
  if ($maybe.tag === "Just") {
    return 1 | 0;
  } else {
    throw new Error("Pattern match failure");
  }
}
