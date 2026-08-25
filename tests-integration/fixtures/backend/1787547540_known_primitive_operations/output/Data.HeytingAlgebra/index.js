export function not(dictionary) {
  return dictionary.not;
}

export const heytingAlgebraBoolean = (() => {
  function heytingAlgebraBoolean$initialize$closure(value) {
    if (value) {
      return false;
    } else {
      return true;
    }
  }
  return { not: heytingAlgebraBoolean$initialize$closure };
})();
