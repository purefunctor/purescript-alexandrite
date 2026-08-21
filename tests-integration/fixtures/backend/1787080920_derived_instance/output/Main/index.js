import * as Data_Show from "../Data.Show/index.js";

export const Box = ["Box"];

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

function rendered$initialize$closure(value) {
  return value;
}

export function showIdentity(dictionary1) {
  return dictionary1;
}

export const eqBox = { eq: eqBox$initialize$closure };

export const equal = (0, eqBox.eq)(Box)(Box);

export const rendered = (0, (showIdentity(Data_Show.showInt)).show)(
  rendered$initialize$closure(42 | 0)
);
