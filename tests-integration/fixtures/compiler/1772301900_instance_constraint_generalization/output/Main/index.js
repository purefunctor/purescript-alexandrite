export function eq(dictionary) {
  return dictionary.eq;
}
export function compare(dictionary) {
  return dictionary.compare;
}
export function test1(eqDict) {
  const $closure = (x) => {
    if (/* @__PURE__ */ eq(eqDict)(x)(x)) {
      return /* @__PURE__ */ eq(eqDict)(x)(x);
    } else {
      return false;
    }
  };
  return $closure;
}
export function test2(ordDict) {
  return (eqDict) => {
    const $closure = (x) => {
      if (/* @__PURE__ */ eq(eqDict)(x)(x)) {
        return /* @__PURE__ */ compare(ordDict)(x)(x);
      } else {
        return /* @__PURE__ */ compare(ordDict)(x)(x);
      }
    };
    return $closure;
  };
}
