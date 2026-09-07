export function map(dictionary) {
  return dictionary.map;
}
export function functorWrapType(functorWDict) {
  return { map: (f) => (x) => /* @__PURE__ */ map(functorWDict)(f)(x) };
}
