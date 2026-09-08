export function pure(dictionary) {
  return dictionary.pure;
}
export function bind(dictionary) {
  return dictionary.bind;
}
export function discard(dictionary) {
  return dictionary.discard;
}
export function usingBind(operationsEffectDict) {
  return /* @__PURE__ */ bind(operationsEffectDict)(/* @__PURE__ */ pure(operationsEffectDict)(1 | 0))((value) => /* @__PURE__ */ pure(operationsEffectDict)(value));
}
export function usingDiscard(operationsEffectDict) {
  return /* @__PURE__ */ discard(operationsEffectDict)(/* @__PURE__ */ pure(operationsEffectDict)("ignored"))(($string) => /* @__PURE__ */ pure(operationsEffectDict)(1 | 0));
}
