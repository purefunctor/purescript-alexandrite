export function eq(dictionary) {
  return dictionary.eq;
}
export function test(eqADict) {
  return (a) => /* @__PURE__ */ eq(eqADict)(a)(a);
}
