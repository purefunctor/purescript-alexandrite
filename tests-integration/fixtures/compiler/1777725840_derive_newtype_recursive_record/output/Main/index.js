import * as Data_Eq from "../Data.Eq/index.js";
export const Nothing = "Nothing";
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export function eqMaybe(eqADict) {
  return { eq: ($maybe) => ($maybe$1) => true };
}
export function eqListType(eqADict) {
  return /* @__PURE__ */ Data_Eq.eqRec({})(/* @__PURE__ */ Data_Eq.eqRowCons(/* @__PURE__ */ Data_Eq.eqRowCons(Data_Eq.eqRowNil)({})({ reflectSymbol: ($proxy) => "tail" })(/* @__PURE__ */ eqMaybe(/* @__PURE__ */ eqListType(eqADict))))({})({ reflectSymbol: ($proxy) => "head" })(Data_Eq.eqInt));
}
