import * as Data_Eq from "../Data.Eq/index.js";
import * as $foreign from "./foreign.js";

export const Just = $value0 => ["Just", $value0];
const Nothing = ["Nothing"];

export function visible(value) {
  return value;
}

export function append(left) {
  return $int => {
    return left;
  };
}

export function measure(dictionary) {
  return dictionary.measure;
}

export const foreignValue = $foreign["foreignValue"];

const hidden = 13 | 0;

const $await = 17 | 0;

export const eqOption = (() => {
  function eqOption$initialize$closure(left) {
    return right => {
      function case$1(left, right) {
        if (Array.isArray(left) && left[0] === "Nothing") {
          if (Array.isArray(right) && right[0] === "Nothing") {
            return true;
          } else {
            return case$2();
          }
        } else {
          return case$2();
        }
      }

      function case$2() {
        return false;
      }

      function if$join(result$1) {
        return result$1;
      }

      if (Array.isArray(left) && left[0] === "Just") {
        const left0 = left[1];
        if (Array.isArray(right) && right[0] === "Just") {
          const right0 = right[1];
          if (Data_Eq.eq(Data_Eq.eqInt)(left0)(right0)) {
            return if$join(true);
          } else {
            return if$join(false);
          }
        } else {
          return case$1(left, right);
        }
      } else {
        return case$1(left, right);
      }
    };
  }
  return { eq: eqOption$initialize$closure };
})();

export const measureInt = (() => {
  return { measure: value => value };
})();

export { $await as "await" };
