import * as Data_Eq from "../Data.Eq/index.js";
import * as $foreign from "./foreign.js";

export const Just = $value0 => ["Just", $value0];
const Nothing = ["Nothing"];

function eqOption$initialize$closure(left) {
  return right => {
    function case$1(left, right) {
      const matches$2 = Array.isArray(left) && left[0] === "Nothing";
      if (matches$2) {
        const matches$3 = Array.isArray(right) && right[0] === "Nothing";
        if (matches$3) {
          const literal$2 = true;
          return literal$2;
        } else {
          return case$2();
        }
      } else {
        return case$2();
      }
    }

    function case$2() {
      const literal$3 = false;
      return literal$3;
    }

    function if$join(result$1) {
      return result$1;
    }

    const matches = Array.isArray(left) && left[0] === "Just";
    if (matches) {
      const left0 = left[1];
      const matches$1 = Array.isArray(right) && right[0] === "Just";
      if (matches$1) {
        const right0 = right[1];
        const eqInt = Data_Eq.eqInt;
        const eq = eqInt.eq;
        const call = eq(left0);
        const call$1 = call(right0);
        if (call$1) {
          const literal = true;
          return if$join(literal);
        } else {
          const literal$1 = false;
          return if$join(literal$1);
        }
      } else {
        return case$1(left, right);
      }
    } else {
      return case$1(left, right);
    }
  };
}

function measureInt$initialize$closure(value) {
  return value;
}

export function visible(value) {
  return value;
}

export function append(left) {
  return argument1 => {
    return left;
  };
}

export const foreignValue = $foreign["foreignValue"];

const hidden = 13 | 0;

const $await = 17 | 0;

export const eqOption = { eq: eqOption$initialize$closure };

export const measureInt = { measure: measureInt$initialize$closure };

export { $await as "await" };
