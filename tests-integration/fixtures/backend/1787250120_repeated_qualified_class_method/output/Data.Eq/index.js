function eqInt$initialize$closure(argument0) {
  return argument1 => {
    return true;
  };
}

function eqBoolean$initialize$closure(argument0) {
  return argument1 => {
    return true;
  };
}

function eqArray$closure(argument0) {
  return argument1 => {
    return true;
  };
}

function orderedInt$initialize$closure(argument0) {
  return argument1 => {
    return true;
  };
}

export function eqArray(dictionary0) {
  return { eq: eqArray$closure };
}

export const eqInt = { eq: eqInt$initialize$closure };

export const eqBoolean = { eq: eqBoolean$initialize$closure };

export const orderedInt = { superclass62: eqInt, lessThanOrEqual: orderedInt$initialize$closure };
