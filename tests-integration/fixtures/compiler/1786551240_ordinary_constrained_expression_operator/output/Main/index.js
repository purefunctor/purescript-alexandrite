export function equal(dictionary) {
  return dictionary.equal;
}
export const equalInt = { equal: (left) => (right) => true };
export const ordinaryConstrainedOperator = /* @__PURE__ */ equal(equalInt)(1 | 0)(2 | 0);
