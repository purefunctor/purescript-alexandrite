export function append(dictionary) {
  return dictionary.append;
}
export const semigroupString = { append: (left) => (right) => left };
const semigroupStringDictAppend = /* @__PURE__ */ append(semigroupString);
export const ordinaryConstrainedOperatorChain = /* @__PURE__ */ semigroupStringDictAppend("first")(/* @__PURE__ */ semigroupStringDictAppend("second")("third"));
