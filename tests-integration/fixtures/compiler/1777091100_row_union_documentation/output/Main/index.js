export const Proxy = "Proxy";
export function closedLeftOpenRight(unionRowRowUDict) {
  return "Proxy";
}
export function closedLeftKnownFields(unionRowRowUDict) {
  return "Proxy";
}
export function closedLeftEmpty(unionRowRowUDict) {
  return "Proxy";
}
export function closedRightOutputOpenLeft(unionLRowRowDict) {
  return "Proxy";
}
export function closedRightOutputKnownLeft(unionRowRowRowDict) {
  return "Proxy";
}
export function closedRightOutputDuplicate(unionLRowRowDict) {
  return "Proxy";
}
export function openLeftPrefixOne(unionRowRowUDict) {
  return "Proxy";
}
export function openLeftPrefixMany(unionRowRowUDict) {
  return "Proxy";
}
export function forceSolve(unionRowDict) {
  return (unionRowDict$1) => {
    return {
      closedLeftOpenRight: /* @__PURE__ */ closedLeftOpenRight({}),
      closedLeftKnownFields: /* @__PURE__ */ closedLeftKnownFields({}),
      closedLeftEmpty: /* @__PURE__ */ closedLeftEmpty({}),
      closedRightOutputOpenLeft: /* @__PURE__ */ closedRightOutputOpenLeft({}),
      closedRightOutputKnownLeft: /* @__PURE__ */ closedRightOutputKnownLeft({}),
      closedRightOutputDuplicate: /* @__PURE__ */ closedRightOutputDuplicate({}),
      openLeftPrefixOne: /* @__PURE__ */ openLeftPrefixOne({}),
      openLeftPrefixMany: /* @__PURE__ */ openLeftPrefixMany({})
    };
  };
}
