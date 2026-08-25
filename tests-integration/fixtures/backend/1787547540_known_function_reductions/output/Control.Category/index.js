export function identity(dictionary) {
  return dictionary.identity;
}

export const categoryFn = (() => {
  return { identity: value => value };
})();
