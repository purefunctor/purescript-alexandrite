import * as Data_Eq from "../Data.Eq/index.js";
import * as Data_Ord from "../Data.Ord/index.js";
export function eqOpaque(eqADict) {
  return { eq: ($opaque) => ($opaque$1) => true };
}
export function ordOpaque(ordADict) {
  return {
    Eq0: () => /* @__PURE__ */ eqOpaque(/* @__PURE__ */ ordADict.Eq0()),
    compare: ($opaque) => ($opaque$1) => "EQ"
  };
}
export const eq1Opaque = { eq1: (eqADict) => /* @__PURE__ */ Data_Eq.eq(/* @__PURE__ */ eqOpaque(eqADict)) };
export const ord1Opaque = {
  Eq10: () => eq1Opaque,
  compare1: (ordADict) => /* @__PURE__ */ Data_Ord.compare(/* @__PURE__ */ ordOpaque(ordADict))
};
