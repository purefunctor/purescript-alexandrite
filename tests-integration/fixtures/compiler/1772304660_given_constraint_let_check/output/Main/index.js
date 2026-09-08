export function eq(dictionary) {
  return dictionary.eq;
}
export const test = /* @__PURE__ */ (() => {
  const impl = (eqADict) => {
    return (a) => /* @__PURE__ */ eq(eqADict)(a)(a);
  };
  return 42 | 0;
})();
