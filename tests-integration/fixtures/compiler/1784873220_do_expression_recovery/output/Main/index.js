import * as $foreign from "./foreign.js";
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const emptyDo = (() => {
  throw new Error("Generated code reached a source error");
})();
export const finalBind = pure(1 | 0);
export const finalLet = (() => {
  const value = 1 | 0;
  throw new Error("Generated code reached a source error");
})();
export const missingDoAction = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  return $result((value) => pure(value));
})();
export const missingFinalBindAction = (() => {
  throw new Error("Generated code reached a source error");
})();
export const missingFinalDiscardAction = (() => {
  throw new Error("Generated code reached a source error");
})();
