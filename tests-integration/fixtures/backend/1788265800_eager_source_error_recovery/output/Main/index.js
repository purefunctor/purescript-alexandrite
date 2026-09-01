import * as $foreign from "./foreign.js";
export const observe = $foreign["observe"];
export const broken = /* @__PURE__ */ (() => {
  const $element = observe(1 | 0);
  let $result;
  throw new Error("Generated code reached a source error");
  return [$element, $result];
})();
