import * as $foreign from "./foreign.js";
export function merge(unionR1R2R3Dict) {
  return ($record) => ($record$1) => unsafeCoerce({});
}
export function merge2(mergeR1R2R3Dict) {
  return (r1) => (r2) => /* @__PURE__ */ merge(/* @__PURE__ */ mergeR1R2R3Dict.Union0())(r1)(r2);
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
