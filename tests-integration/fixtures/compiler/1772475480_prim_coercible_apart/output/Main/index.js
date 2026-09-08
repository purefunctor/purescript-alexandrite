import * as Safe_Coerce from "../Safe.Coerce/index.js";
export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Left = ($value0) => ({
  tag: "Left",
  _1: $value0
});
export const Right = ($value0) => ({
  tag: "Right",
  _1: $value0
});
export const coerceDifferent = (() => {
  const $function = Safe_Coerce.coerce;
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ $function($result);
})();
