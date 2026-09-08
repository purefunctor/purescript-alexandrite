export function append(dictionary) {
  return dictionary.append;
}
export function test(semigroupADict) {
  return (value) => /* @__PURE__ */ append(semigroupADict)(value)(/* @__PURE__ */ test(semigroupADict)(value));
}
