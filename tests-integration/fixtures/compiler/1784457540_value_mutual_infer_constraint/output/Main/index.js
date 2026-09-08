export function append(dictionary) {
  return dictionary.append;
}
export function first(semigroupDict) {
  return (value) => /* @__PURE__ */ append(semigroupDict)(value)(second(value));
}
export function second(semigroupDict) {
  return (value) => /* @__PURE__ */ append(semigroupDict)(value)(first(value));
}
