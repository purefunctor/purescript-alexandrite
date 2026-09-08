export const Proxy = "Proxy";
export function openLeft(unionRowRowUDict) {
  return "Proxy";
}
export function openRight(unionRowRowUDict) {
  return "Proxy";
}
export function backwardLeft(unionLRowRowDict) {
  return "Proxy";
}
export function backwardRight(unionRowRRowDict) {
  return "Proxy";
}
export function forceSolve(unionRowRowDict) {
  return (unionRowDict) => {
    return {
      openLeft: /* @__PURE__ */ openLeft({}),
      openRight: /* @__PURE__ */ openRight({}),
      backwardLeft: /* @__PURE__ */ backwardLeft(unionRowRowDict),
      backwardRight: /* @__PURE__ */ backwardRight({})
    };
  };
}
