export function eq(dictionary) {
  return dictionary.eq;
}
export function eqArray(eqADict) {
  return { eq: ($array) => ($array$1) => false };
}
export const eqInt = { eq: ($int) => ($int$1) => true };
export const test = /* @__PURE__ */ eq(/* @__PURE__ */ eqArray(/* @__PURE__ */ eqArray(eqInt)))([[1 | 0]])([[2 | 0]]);
