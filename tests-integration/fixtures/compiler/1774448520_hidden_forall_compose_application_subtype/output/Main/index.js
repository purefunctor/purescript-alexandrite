export function compose(dictionary) {
  return dictionary.compose;
}
export function make(n) {
  return n;
}
export const semigroupoidFn = { compose: (f) => (g) => (x) => f(g(x)) };
export const test = /* @__PURE__ */ compose(semigroupoidFn)((value) => value)(make);
