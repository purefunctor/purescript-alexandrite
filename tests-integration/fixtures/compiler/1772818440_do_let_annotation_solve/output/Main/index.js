import * as $foreign from "./foreign.js";
export const unit = $foreign["unit"];
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const thing1 = pure("hello");
export const test = /* @__PURE__ */ (() => {
  const $closure = (a) => {
    const f = a;
    return pure(unit);
  };
  return bind(thing1)($closure);
})();
