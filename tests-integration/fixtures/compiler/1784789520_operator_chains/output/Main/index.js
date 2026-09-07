import * as $foreign from "./foreign.js";
export function add(left) {
  return (right) => {
    return left;
  };
}
export function multiply(left) {
  return (right) => {
    return right;
  };
}
export function append(left) {
  return (right) => {
    return left;
  };
}
export function implicitApplications(firstADict) {
  return (secondBDict) => {
    return (left) => (right) => /* @__PURE__ */ interleaved(firstADict)(left)(secondBDict)(right);
  };
}
export const interleaved = $foreign["interleaved"];
export const precedence = add(1 | 0)(multiply(2 | 0)(3 | 0));
export const leftAssociative = add(add(1 | 0)(2 | 0))(3 | 0);
export const rightAssociative = append(1 | 0)(append(2 | 0)(3 | 0));
