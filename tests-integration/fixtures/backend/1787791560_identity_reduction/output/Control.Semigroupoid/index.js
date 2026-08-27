export function compose(dictionary) {
  return dictionary.compose;
}
export const semigroupoidFn = { compose: (outer) => (inner) => (value) => outer(inner(value)) };
