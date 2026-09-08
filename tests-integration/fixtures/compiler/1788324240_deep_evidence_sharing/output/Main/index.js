import * as $foreign from "./foreign.js";
export const Proxy = "Proxy";
export function chain(dictionary) {
  return dictionary.chain;
}
export function chainNext(chainPreviousDict) {
  return { chain: observe(0 | 0) };
}
export function pair(dictionary) {
  return dictionary.pair;
}
export function pairInstance(chainNumberDict) {
  return (chainNumberDict$1) => {
    return { pair: ($proxy) => 0 | 0 };
  };
}
export const observe = $foreign["observe"];
export const chainZero = { chain: 0 | 0 };
export const result = /* @__PURE__ */ (() => {
  const chainNextDict = /* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(chainZero)))))))))))))))))))))))))))))));
  const chainNextDict$1 = /* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(/* @__PURE__ */ chainNext(chainNextDict)))))))));
  const $result = /* @__PURE__ */ pairInstance(chainNextDict$1)(chainNextDict$1);
  return /* @__PURE__ */ pair($result)("Proxy");
})();
