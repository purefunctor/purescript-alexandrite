import * as Data_Eq from "../Data.Eq/index.js";
import * as $foreign from "./foreign.js";
export const Just = ($value0) => ["Just", $value0];
const Nothing = "Nothing";
export function visible(value) {
  return value;
}
export function append(left) {
  return ($int) => {
    return left;
  };
}
export function measure(dictionary) {
  return dictionary.measure;
}
export const foreignValue = $foreign["foreignValue"];
const hidden = 13 | 0;
const $await = 17 | 0;
export const eqOption = /* @__PURE__ */ (() => {
  const $closure = (left) => {
    return (right) => {
      if (Array.isArray(left) && left[0] === "Just" && Array.isArray(right) && right[0] === "Just") {
        const left0 = left[1];
        const right0 = right[1];
        if (/* @__PURE__ */ Data_Eq.eq(Data_Eq.eqInt)(left0)(right0)) {
          return true;
        } else {
          return false;
        }
      }
      if (left === "Nothing" && right === "Nothing") {
        return true;
      }
      return false;
    };
  };
  return { eq: $closure };
})();
export const measureInt = { measure: (value) => value };
export { $await as "await" };
