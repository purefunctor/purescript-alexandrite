export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export function append(dictionary) {
  return dictionary.append;
}
export function test(semigroupADict) {
  return (semigroupBDict) => {
    return (a1) => (a2) => (b1) => (b2) => ({
      tag: "Tuple",
      _1: /* @__PURE__ */ append(semigroupADict)(a1)(a2),
      _2: /* @__PURE__ */ append(semigroupBDict)(b1)(b2)
    });
  };
}
