import * as Safe_Coerce from "../Safe.Coerce/index.js";
export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nil = "Nil";
export const Cons = ($value0) => ($value1) => ({
  tag: "Cons",
  _1: $value0,
  _2: $value1
});
export const coerceContainerDifferent = /* @__PURE__ */ Safe_Coerce.coerce({});
