export function recordDef(dictionary) {
  return dictionary.recordDef;
}
export function recordDefRowList(dictionary) {
  return dictionary.recordDefRowList;
}
export function recordDef1(recordDefRowListTokenRowRowListDict) {
  return { recordDef: ($interface) => (token) => (row) => /* @__PURE__ */ recordDefRowList(recordDefRowListTokenRowRowListDict)($interface)(token)(row)("Proxy") };
}
