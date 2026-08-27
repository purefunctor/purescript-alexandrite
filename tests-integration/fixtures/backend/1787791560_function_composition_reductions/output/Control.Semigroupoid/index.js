export function compose(dictionary) {
  return dictionary.compose;
}
export function composeFlipped(semigroupoidSemigroupoidDict) {
  return (inner) => (outer) => /* @__PURE__ */ compose(semigroupoidSemigroupoidDict)(outer)(inner);
}
export const semigroupoidFn = { compose: (outer) => (inner) => (value) => outer(inner(value)) };
