export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test(section15) {
  if (section15.tag === "Just") {
    return 1 | 0;
  }
  if (section15.tag === "Just") {
    return 2 | 0;
  }
  if (section15 === "Nothing") {
    return 3 | 0;
  }
  throw new Error("Pattern match failure");
}
