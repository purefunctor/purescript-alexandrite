export function name(dictionary) {
  return dictionary.name;
}
export function use(namedADict) {
  return (value) => /* @__PURE__ */ name(namedADict)(value);
}
export function useLet(namedADict) {
  return (value) => /* @__PURE__ */ name(namedADict)(value);
}
export const namedString = { name: (value) => value };
