import * as $foreign from "./foreign.js";
export function equal(dictionary) {
  return dictionary.equal;
}
export function lessThan(dictionary) {
  return dictionary.lessThan;
}
export function genericEqual(equalADict) {
  return (left) => (right) => /* @__PURE__ */ equal(equalADict)(left)(right);
}
export function superclassEqual(orderedADict) {
  return (left) => (right) => /* @__PURE__ */ equal(/* @__PURE__ */ orderedADict.Equal0())(left)(right);
}
export const equalInt = $foreign["equalInt"];
export const lessThanInt = $foreign["lessThanInt"];
export const equalInt1 = { equal: equalInt };
export const orderedInt = {
  Equal0: () => equalInt1,
  lessThan: lessThanInt
};
export const concreteEqual = /* @__PURE__ */ equal(equalInt1)(1 | 0)(2 | 0);
export const concreteLessThan = /* @__PURE__ */ lessThan(orderedInt)(1 | 0)(2 | 0);
