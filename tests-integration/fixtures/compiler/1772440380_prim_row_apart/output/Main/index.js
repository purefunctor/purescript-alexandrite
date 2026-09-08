export function unionTypeMismatch(unionLRowRowDict) {
  return "Proxy";
}
export function consMissingLabel(consIntTRowDict) {
  return "Proxy";
}
export function lacksPresent(lacksRowDict) {
  return "Proxy";
}
export const forceSolve = (() => {
  const $field = /* @__PURE__ */ unionTypeMismatch({});
  const $field$1 = /* @__PURE__ */ consMissingLabel({});
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    unionTypeMismatch: $field,
    consMissingLabel: $field$1,
    lacksPresent: /* @__PURE__ */ lacksPresent($result)
  };
})();
