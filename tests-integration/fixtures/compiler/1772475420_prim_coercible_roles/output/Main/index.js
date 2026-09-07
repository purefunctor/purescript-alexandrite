import * as Safe_Coerce from "../Safe.Coerce/index.js";
export const Proxy = "Proxy";
export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const coerceProxy = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceMaybe = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceMaybeReverse = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceRecord = /* @__PURE__ */ Safe_Coerce.coerce({});
export const coerceFunction = /* @__PURE__ */ Safe_Coerce.coerce({});
