export function map(dictionary) {
  return dictionary.map;
}
export const functorFn = { map: (f) => (g) => (x) => f(g(x)) };
