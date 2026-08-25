export function eq(dictionary) {
  return dictionary.eq;
}

export const eqInt = (() => {
  return { eq: $int => $int$1 => true };
})();
