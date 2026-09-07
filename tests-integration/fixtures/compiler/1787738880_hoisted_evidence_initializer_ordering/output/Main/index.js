import * as Data_Eq from "../Data.Eq/index.js";
export function compareInts(left) {
  return (right) => {
    if (/* @__PURE__ */ eqIntDictEq(left)(right)) {
      return /* @__PURE__ */ eqIntDictEq(right)(left);
    } else {
      return false;
    }
  };
}
const eqIntDictEq = /* @__PURE__ */ Data_Eq.eq(Data_Eq.eqInt);
export const initialized = compareInts(1 | 0)(1 | 0);
