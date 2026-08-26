import * as Data_Eq from "../Data.Eq/index.js";

export function compareArraysOnce(left) {
  return right => {
    return Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
  };
}
