export function eq(dictionary) {
  return dictionary.eq;
}

export function eqArray(eqADict) {
  return { eq: $array => $array$1 => true };
}

export const eqInt = { eq: $int => $int$1 => true };
