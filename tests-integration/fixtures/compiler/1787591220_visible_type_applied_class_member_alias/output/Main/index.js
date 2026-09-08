export function name(dictionary) {
  return dictionary.name;
}
export function use(namedBodyDict) {
  return ($body) => /* @__PURE__ */ name(namedBodyDict);
}
