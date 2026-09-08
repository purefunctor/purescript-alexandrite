export function name(dictionary) {
  return dictionary.name;
}
export function select(namedSelectedDict) {
  return /* @__PURE__ */ name(namedSelectedDict);
}
export const namedTypeString = { name: "x" };
export const test = /* @__PURE__ */ select(namedTypeString);
