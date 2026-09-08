import * as $foreign from "./foreign.js";
export function union(unionR1R2R3Dict) {
  return ($record) => ($record$1) => unsafeCoerce({});
}
export function addField(x) {
  return /* @__PURE__ */ union({})(x)({ b: "hi" });
}
export function insertX(lacksRDict) {
  return ($record) => unsafeCoerce({});
}
export function insertOpen(lacksRDict) {
  return (x) => /* @__PURE__ */ insertX({})(x);
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
export const test = addField({
  a: 1 | 0,
  c: true
});
export const test2 = /* @__PURE__ */ insertOpen({})({
  a: 1 | 0,
  b: "hi"
});
