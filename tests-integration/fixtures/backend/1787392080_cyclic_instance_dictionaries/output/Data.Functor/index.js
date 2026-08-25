export function map(dictionary) {
  return dictionary.map;
}

export const functorFn = (() => {
  return { map: f => g => x => f(g(x)) };
})();
