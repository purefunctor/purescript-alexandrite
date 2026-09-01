export function bind(dictionary) {
  return dictionary.bind;
}
export function discard(dictionary) {
  return dictionary.discard;
}
export const discardUnit = { discard: (bindFDict) => /* @__PURE__ */ bind(bindFDict) };
