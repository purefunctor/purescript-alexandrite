export function partialValue(partialDict) {
  return 0 | 0;
}
export function test(partialDict) {
  if (true) {
    return test;
  } else {
    return /* @__PURE__ */ partialValue(partialDict);
  }
}
