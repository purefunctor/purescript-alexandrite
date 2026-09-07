export function mk(consSAR_CombinedRowDict) {
  return ($proxy) => ($a) => "Proxy";
}
export const test = /* @__PURE__ */ mk({})("Proxy")("");
export const forceSolve = { test };
