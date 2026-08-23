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

export const equal = eqBox.eq(Box)(Box);

export const rendered = (showIdentity(Data_Show.showInt)).show(42 | 0);
