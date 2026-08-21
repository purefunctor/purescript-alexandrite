export function eqArray(dictionary0) {
  function eqArray$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { eq: eqArray$closure };
}

export const eqInt = (() => {
  function eqInt$initialize$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { eq: eqInt$initialize$closure };
})();

export const eqBoolean = (() => {
  function eqBoolean$initialize$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { eq: eqBoolean$initialize$closure };
})();

export const orderedInt = (() => {
  function orderedInt$initialize$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { superclass62: eqInt, lessThanOrEqual: orderedInt$initialize$closure };
})();
