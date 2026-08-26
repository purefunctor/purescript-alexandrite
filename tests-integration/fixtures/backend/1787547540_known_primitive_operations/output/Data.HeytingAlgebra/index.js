export function not(dictionary) {
  return dictionary.not;
}
export const heytingAlgebraBoolean = (() => {
  const $closure = (value) => {
    if (value) {
      return false;
    } else {
      return true;
    }
  };
  return { not: $closure };
})();
