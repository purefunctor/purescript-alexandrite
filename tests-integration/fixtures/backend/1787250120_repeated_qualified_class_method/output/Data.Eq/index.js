export function eq(dictionary) {
  return dictionary.eq;
}

export function eqArray(eqADict) {
  return { eq: $array => $array$1 => true };
}

export function lessThanOrEqual(dictionary) {
  return dictionary.lessThanOrEqual;
}

export const eqInt = (() => {
  return { eq: $int => $int$1 => true };
})();

export const eqBoolean = (() => {
  return { eq: $boolean => $boolean$1 => true };
})();

export const orderedInt = (() => {
  return { Eq0: () => eqInt, lessThanOrEqual: $int => $int$1 => true };
})();
