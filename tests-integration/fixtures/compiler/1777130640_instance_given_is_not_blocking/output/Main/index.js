import * as $foreign from "./foreign.js";
export function buildIt(dictionary) {
  return dictionary.buildIt;
}
export function test(buildRecordRSDict) {
  return /* @__PURE__ */ buildIt(buildRecordImpl);
}
export const unsafeSet = $foreign["unsafeSet"];
export const buildRecordImpl = /* @__PURE__ */ (() => {
  const result = unsafeSet("x")(42 | 0)({});
  const $result = result;
  return { buildIt: $result };
})();
