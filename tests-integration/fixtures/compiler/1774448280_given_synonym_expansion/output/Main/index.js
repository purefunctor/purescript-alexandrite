import * as $foreign from "./foreign.js";
export function coerceRecord(typeEqualsABDict) {
  return unsafeCoerce;
}
export function transform(typeEqualsAIncludeDict) {
  return (v) => /* @__PURE__ */ coerceRecord(typeEqualsAIncludeDict)(v);
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
export const typeEquals = {};
export const test = /* @__PURE__ */ transform(typeEquals)({
  x: 42 | 0,
  y: "life",
  z: false
});
