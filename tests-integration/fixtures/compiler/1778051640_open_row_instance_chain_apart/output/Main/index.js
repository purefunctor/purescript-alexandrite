export const Proxy = "Proxy";
export const K1 = "K1";
export const Unit = "Unit";
export function c(dictionary) {
  return dictionary.c;
}
export function test(key) {
  return /* @__PURE__ */ c(cRow)(key)("Proxy");
}
export const cK1Row = { c: ($proxy) => ($proxy$1) => "Unit" };
export const cRow = { c: ($proxy) => ($proxy$1) => "Unit" };
