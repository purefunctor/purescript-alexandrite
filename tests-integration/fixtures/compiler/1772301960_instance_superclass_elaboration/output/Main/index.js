export function eq(dictionary) {
  return dictionary.eq;
}
export function compare(dictionary) {
  return dictionary.compare;
}
export function test(ordDict) {
  const $closure = (x) => {
    if (/* @__PURE__ */ eq(/* @__PURE__ */ ordDict.Eq0())(x)(x)) {
      return /* @__PURE__ */ compare(ordDict)(x)(x);
    } else {
      return /* @__PURE__ */ compare(ordDict)(x)(x);
    }
  };
  return $closure;
}
