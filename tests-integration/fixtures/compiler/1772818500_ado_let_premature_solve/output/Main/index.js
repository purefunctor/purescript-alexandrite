import * as $foreign from "./foreign.js";
export function add(dictionary) {
  return dictionary.add;
}
export const unit = $foreign["unit"];
export const pure = $foreign["pure"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const addImpl = $foreign["addImpl"];
export const semiringInt = { add: addImpl };
export const thing1 = pure("hello");
export const test = /* @__PURE__ */ (() => {
  const $closure = (a) => {
    let $result;
    throw new Error("Generated code reached a source error");
    const f = /* @__PURE__ */ add($result)(a)(123 | 0);
    return unit;
  };
  return map($closure)(thing1);
})();
