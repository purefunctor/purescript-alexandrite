export function eqArray(eqADict) {
  return { eq: $array => $array$1 => true };
}

export const eqInt = (() => {
  return { eq: $int => $int$1 => true };
})();

export const eqBoolean = (() => {
  return { eq: $boolean => $boolean$1 => true };
})();

export const orderedInt = (() => {
  return { Eq0: unit => eqInt, lessThanOrEqual: $int => $int$1 => true };
})();
