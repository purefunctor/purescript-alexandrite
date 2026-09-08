import * as $foreign from "./foreign.js";
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const test = (() => {
  throw new Error("Generated code reached a source error");
})();
