export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export function pure(dictionary) {
  return dictionary.pure;
}
export function map(dictionary) {
  return dictionary.map;
}
export function apply(dictionary) {
  return dictionary.apply;
}
export function usingMapApply(operationsEffectDict) {
  return /* @__PURE__ */ apply(operationsEffectDict)(/* @__PURE__ */ map(operationsEffectDict)((first) => (second) => ({
    tag: "Tuple",
    _1: first,
    _2: second
  }))(/* @__PURE__ */ pure(operationsEffectDict)(1 | 0)))(/* @__PURE__ */ pure(operationsEffectDict)("two"));
}
export function usingPure(operationsEffectDict) {
  return /* @__PURE__ */ pure(operationsEffectDict)(1 | 0);
}
