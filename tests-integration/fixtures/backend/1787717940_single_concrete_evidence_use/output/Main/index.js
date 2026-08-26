import * as Data_Eq from "../Data.Eq/index.js";
export function compareArraysOnce(left) {
  return (right) => {
    return /* @__PURE__ */ Data_Eq.eq(/* @__PURE__ */ Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
  };
}
