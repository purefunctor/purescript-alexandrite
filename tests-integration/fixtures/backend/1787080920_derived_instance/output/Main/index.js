import * as Data_Show from "../Data.Show/index.js";

export const Box = ["Box"];

function eqBox$initialize$closure(left) {
  return right => {
    const matches = Array.isArray(left) && left[0] === "Box";
    if (matches) {
      const matches$1 = Array.isArray(right) && right[0] === "Box";
      if (matches$1) {
        const literal = true;
        return literal;
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

export const equal = eqBox.eq(Box)(Box);

export const rendered = (showIdentity(Data_Show.showInt)).show(rendered$initialize$closure(42 | 0));
