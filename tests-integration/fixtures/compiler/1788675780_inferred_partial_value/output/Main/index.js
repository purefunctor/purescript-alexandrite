import * as $foreign from "./foreign.js";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function test(partialDict) {
  const $scrutinee = {
    tag: "Just",
    _1: 1 | 0
  };
  if ($scrutinee.tag === "Just") {
    const { _1: x } = $scrutinee;
    return x;
  } else {
    throw new Error("Pattern match failure");
  }
}
export function consumed(partialDict) {
  return /* @__PURE__ */ test(partialDict);
}
export const unsafePartial = $foreign["unsafePartial"];
export const discharged = unsafePartial((partialDict) => /* @__PURE__ */ test(partialDict));
