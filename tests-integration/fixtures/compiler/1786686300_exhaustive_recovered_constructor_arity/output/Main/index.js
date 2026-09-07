export const Box = ($value0) => ({
  tag: "Box",
  _1: $value0
});
export const Other = "Other";
export function test(partialDict) {
  const $closure = ($box) => {
    if ($box.tag === "Box") {
      return 0 | 0;
    }
    if ($box === "Box") {
      return 1 | 0;
    }
    throw new Error("Pattern match failure");
  };
  return $closure;
}
export function test2(partialDict) {
  const $closure = ($function) => {
    if ($function === "Box") {
      return 0 | 0;
    }
    if ($function.tag === "Box") {
      return 1 | 0;
    }
    throw new Error("Pattern match failure");
  };
  return $closure;
}
