export function convert(dictionary) {
  return dictionary.convert;
}
export function useGiven(convertIntXDict) {
  return /* @__PURE__ */ convert(convertIntXDict)(42 | 0);
}
export function relate(dictionary) {
  return dictionary.relate;
}
export function useRelate(relateIntStringYDict) {
  return /* @__PURE__ */ relate(relateIntStringYDict)(1 | 0)("hello");
}
