export function assertGreater(compareLRGTDict) {
  return "Proxy";
}
export function assertLesser(compareLRLTDict) {
  return "Proxy";
}
export function weakenZeroToNegOne(compareHGTDict) {
  return /* @__PURE__ */ assertGreater({});
}
export function weakenZeroToNegTwo(compareHGTDict) {
  return /* @__PURE__ */ assertGreater({});
}
export function weakenZeroLT(compareHLTDict) {
  return /* @__PURE__ */ assertLesser({});
}
export function chainAbstractAndConcrete(compareHKGTDict) {
  return (compareKGTDict) => {
    return /* @__PURE__ */ assertGreater({});
  };
}
