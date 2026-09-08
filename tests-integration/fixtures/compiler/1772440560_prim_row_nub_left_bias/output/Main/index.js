import * as $foreign from "./foreign.js";
export function merge(unionR1R2R3Dict) {
  return (nubR3R4Dict) => {
    return ($record) => ($record$1) => unsafeCoerce({});
  };
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
export const test = /* @__PURE__ */ merge({})({})({
  a: 42 | 0,
  b: "life"
})({ b: 42 | 0 });
