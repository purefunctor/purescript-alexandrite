export function append(dictionary) {
  return dictionary.append;
}
export function test(semigroupDict) {
  return (value) => /* @__PURE__ */ append(semigroupDict)(value)(test(value));
}
