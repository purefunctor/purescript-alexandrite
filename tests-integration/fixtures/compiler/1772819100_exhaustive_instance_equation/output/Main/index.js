export function c(dictionary) {
  return dictionary.c;
}
export const cBool = /* @__PURE__ */ (() => {
  const $closure = ($boolean) => {
    if ($boolean === true) {
      return 1 | 0;
    } else {
      throw new Error("Pattern match failure");
    }
  };
  return { c: $closure };
})();
