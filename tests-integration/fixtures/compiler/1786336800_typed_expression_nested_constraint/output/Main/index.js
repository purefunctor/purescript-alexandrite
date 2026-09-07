export function constrainedIdentity(valueADict) {
  return (value) => value;
}
export function test(valueADict) {
  return /* @__PURE__ */ ((valueBDict) => /* @__PURE__ */ constrainedIdentity(valueBDict))(valueADict);
}
