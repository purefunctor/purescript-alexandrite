export function show(dictionary) {
  return dictionary.show;
}
export function interleaved(dictionary) {
  return dictionary.interleaved;
}
export const record = { show: (eqADict) => /* @__PURE__ */ show(eqADict) };
export const interleavedRecord = { interleaved: (firstADict) => (secondBDict) => /* @__PURE__ */ interleaved(firstADict)(secondBDict) };
