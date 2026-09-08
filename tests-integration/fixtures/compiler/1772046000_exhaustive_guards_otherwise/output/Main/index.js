import * as Data_Boolean from "../Data.Boolean/index.js";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test1(partialDict) {
  const $closure = (section21) => {
    let $result;
    $case: {
      if (section21.tag === "Just") {
        if (Data_Boolean.otherwise) {
          $result = 1 | 0;
          break $case;
        }
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
export function test2(partialDict) {
  const $closure = (section38) => {
    let $result;
    $case: {
      if (section38 === "Nothing") {
        if (true) {
          $result = 2 | 0;
          break $case;
        }
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
