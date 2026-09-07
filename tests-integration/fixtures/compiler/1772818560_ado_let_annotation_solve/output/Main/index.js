import * as $foreign from "./foreign.js";
export const unit = $foreign["unit"];
export const pure = $foreign["pure"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const thing1 = pure("hello");
export const test = /* @__PURE__ */ (() => {
  const $closure = (a) => {
    const f = a;
    return unit;
  };
  return map($closure)(thing1);
})();
