export function assertLesser(compareLRLTDict) {
  return "Proxy";
}
export function assertGreater(compareLRGTDict) {
  return "Proxy";
}
export function assertEqual(compareLREQDict) {
  return "Proxy";
}
export function transLt(compareMNLTDict) {
  return (compareNPLTDict) => {
    return ($proxy) => /* @__PURE__ */ assertLesser({});
  };
}
export function transLtEq(compareMNLTDict) {
  return (compareNPEQDict) => {
    return ($proxy) => /* @__PURE__ */ assertLesser({});
  };
}
export function transEqGt(compareMNEQDict) {
  return (compareNPGTDict) => {
    return ($proxy) => /* @__PURE__ */ assertGreater({});
  };
}
export function transSymmEq(compareNMEQDict) {
  return (compareNPEQDict) => {
    return ($proxy) => /* @__PURE__ */ assertEqual({});
  };
}
