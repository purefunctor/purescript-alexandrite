export const Cons = ($value0) => ($value1) => ({
  tag: "Cons",
  _1: $value0,
  _2: $value1
});
export const Nil = "Nil";
export function combine(dictionary) {
  return dictionary.combine;
}
export function operatorName(combineADict) {
  return /* @__PURE__ */ combine(combineADict);
}
export function operatorApplication(combineADict) {
  return (left) => (right) => /* @__PURE__ */ combine(combineADict)(left)(right);
}
export const constructorOperator = Cons;
