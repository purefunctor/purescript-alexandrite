export const Proxy = "Proxy";
export function lacksOpen(lacksRowDict) {
  return ($proxy) => 0 | 0;
}
export function lacksPresent(lacksRowDict) {
  return ($proxy) => 0 | 0;
}
export const empty = "Proxy";
export const forceSolve = (() => {
  const $field = /* @__PURE__ */ lacksOpen({})(empty);
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    lacksOpen: $field,
    lacksPresent: /* @__PURE__ */ lacksPresent($result)(empty)
  };
})();
