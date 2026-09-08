export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test(partialDict) {
  const $closure = (section15) => {
    let $result;
    $case: {
      if (section15.tag === "Just") {
        $result = 1 | 0;
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
