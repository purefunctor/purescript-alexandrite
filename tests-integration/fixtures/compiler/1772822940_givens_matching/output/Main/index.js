export function eq(dictionary) {
  return dictionary.eq;
}
export function test(eqADict) {
  return (x) => /* @__PURE__ */ eq(eqADict)(x)(x);
}
