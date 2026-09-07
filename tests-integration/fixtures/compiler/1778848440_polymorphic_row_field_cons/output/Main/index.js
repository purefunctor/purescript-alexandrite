export const Proxy = "Proxy";
export const Html = "Html";
export function slot(consLabelSlotQueryOutputDict) {
  return ($proxy) => ($key) => "Html";
}
export function test(key) {
  return /* @__PURE__ */ slot({})(_child)(key);
}
export const _child = "Proxy";
