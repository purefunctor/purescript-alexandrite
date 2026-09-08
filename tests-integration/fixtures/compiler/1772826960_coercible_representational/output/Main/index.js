import * as Safe_Coerce from "../Safe.Coerce/index.js";
export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const coerceMaybe = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceMaybeReverse = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceNested = /* @__PURE__ */ Safe_Coerce.coerce({});
