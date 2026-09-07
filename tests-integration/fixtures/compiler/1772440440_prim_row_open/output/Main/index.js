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
export function consOpen(consIntRowRowDict) {
  return "Proxy";
}
export function decomposeOpen(consTTailRowDict) {
  return "Proxy";
}
export function extractTail(consIntTailRowDict) {
  return "Proxy";
}
export function lacksOpen(lacksRowDict) {
  return ($proxy) => 0 | 0;
}
export function lacksPresent(lacksRowDict) {
  return ($proxy) => 0 | 0;
}
export function forceSolve(unionRowRowDict) {
  return (unionRowDict) => {
    const $field = /* @__PURE__ */ openLeft({});
    const $field$1 = /* @__PURE__ */ openRight({});
    const $field$2 = /* @__PURE__ */ backwardLeft(unionRowRowDict);
    const $field$3 = /* @__PURE__ */ backwardRight({});
    const $field$4 = /* @__PURE__ */ consOpen({});
    const $field$5 = /* @__PURE__ */ decomposeOpen({});
    const $field$6 = /* @__PURE__ */ extractTail({});
    const $field$7 = /* @__PURE__ */ lacksOpen({})("Proxy");
    let $result;
    throw new Error("Generated code reached a source error");
    return {
      openLeft: $field,
      openRight: $field$1,
      backwardLeft: $field$2,
      backwardRight: $field$3,
      consOpen: $field$4,
      decomposeOpen: $field$5,
      extractTail: $field$6,
      lacksOpen: $field$7,
      lacksPresent: /* @__PURE__ */ lacksPresent($result)("Proxy")
    };
  };
}
