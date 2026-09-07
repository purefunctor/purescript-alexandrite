import * as $foreign from "./foreign.js";
export const combine = $foreign["combine"];
export const constrainedValue = $foreign["constrainedValue"];
export const needsFirst = $foreign["needsFirst"];
export const test = (() => {
  const $function = combine((firstIntDict) => /* @__PURE__ */ constrainedValue(firstIntDict));
  let $result;
  throw new Error("Generated code reached a source error");
  return $function(/* @__PURE__ */ needsFirst($result));
})();
