import * as $foreign from "./foreign.js";
export function merge(unionR1R2R3Dict) {
  return (nubR3R4Dict) => {
    return ($record) => ($record$1) => unsafeCoerce({});
  };
}
export function a(nubRowDict) {
  return /* @__PURE__ */ merge({})(nubRowDict)({ a: 123 | 0 });
}
export function fromUnion(unionR1R2R3Dict) {
  return ($record) => unsafeCoerce(0 | 0);
}
export function test(unionDict) {
  return /* @__PURE__ */ fromUnion(unionDict);
}
export function chainedUnion(unionR1R2R3Dict) {
  return (unionR3R4R5Dict) => {
    return ($record) => unsafeCoerce({});
  };
}
export function testChained(unionDict) {
  return /* @__PURE__ */ chainedUnion({})({})({ x: 1 | 0 });
}
export function multiMerge(unionR1R2R3Dict) {
  return (nubR3R4Dict) => {
    return (unionR4R5R6Dict) => {
      return ($record) => ($record$1) => unsafeCoerce({});
    };
  };
}
export function testMulti1(unionDict) {
  return (nubRowDict) => {
    return /* @__PURE__ */ multiMerge({})(nubRowDict)(unionDict)({ a: 1 | 0 });
  };
}
export function testMulti2(nubRowDict) {
  return (unionRowDict) => {
    return /* @__PURE__ */ multiMerge({})(nubRowDict)(unionRowDict)({ a: 1 | 0 })({ b: 2 | 0 });
  };
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
export const b = /* @__PURE__ */ a({})({ b: 123 | 0 });
