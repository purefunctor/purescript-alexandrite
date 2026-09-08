export function addChain(addNDict) {
  return (addNMDict) => {
    return "Proxy";
  };
}
export function addContradiction(addDict) {
  return (addMDict) => {
    return "Proxy";
  };
}
export const forceSolve = (() => {
  const $field = /* @__PURE__ */ addChain({})({});
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    addChain: $field,
    addContradiction: /* @__PURE__ */ addContradiction($result)({})
  };
})();
