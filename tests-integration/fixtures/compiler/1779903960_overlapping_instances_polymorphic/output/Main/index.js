export const Proxy = "Proxy";
export function showP(dictionary) {
  return dictionary.showP;
}
export const showPFirst = { showP: ($proxy) => "first" };
export const showPSecond = { showP: ($proxy) => "second" };
export const value = /* @__PURE__ */ showP(showPFirst)("Proxy");
