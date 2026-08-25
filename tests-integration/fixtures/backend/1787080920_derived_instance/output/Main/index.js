import * as Data_Eq from "../Data.Eq/index.js";
import * as Data_Show from "../Data.Show/index.js";

export const Box = ["Box"];

export function showIdentity(showADict) {
  return showADict;
}

export const eqBox = (() => {
  function eqBox$initialize$closure(left) {
    return right => {
      if (Array.isArray(left) && left[0] === "Box") {
        if (Array.isArray(right) && right[0] === "Box") {
          return true;
        } else {
          throw new Error("Pattern match failure");
        }
      } else {
        throw new Error("Pattern match failure");
      }
    };
  }
  return { eq: eqBox$initialize$closure };
})();

export const equal = Data_Eq.eq(eqBox)(Box)(Box);

export const rendered = Data_Show.show(showIdentity(Data_Show.showInt))(42 | 0);
