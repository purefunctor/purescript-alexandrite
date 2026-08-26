export function not(dictionary) {
  return dictionary.not;
}
export const heytingAlgebraBoolean = /* @__PURE__ */ (() => {
  const $closure = (value) => {
    if (value) {
      return false;
    } else {
      return true;
    }
  };
  return { not: $closure };
})();
