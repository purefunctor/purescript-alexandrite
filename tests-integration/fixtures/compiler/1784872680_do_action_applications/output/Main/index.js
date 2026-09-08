import * as $foreign from "./foreign.js";
export function selected(selectIntDict) {
  return bind(/* @__PURE__ */ constrained(selectIntDict))((value) => pure(value));
}
export const constrained = $foreign["constrained"];
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
