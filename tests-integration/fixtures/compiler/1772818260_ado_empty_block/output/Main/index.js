import * as $foreign from "./foreign.js";
export const pure = $foreign["pure"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const test1 = (() => {
  throw new Error("Generated code reached a source error");
})();
export const test2 = pure(123 | 0);
