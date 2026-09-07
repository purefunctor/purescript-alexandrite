import * as $foreign from "./foreign.js";
export function negative(negativeADict) {
  return (value) => /* @__PURE__ */ negate(negativeADict)(value);
}
export function shadowedNegate(negate$1) {
  return (value) => {
    return negate$1(value);
  };
}
export const negate = $foreign["negate"];
