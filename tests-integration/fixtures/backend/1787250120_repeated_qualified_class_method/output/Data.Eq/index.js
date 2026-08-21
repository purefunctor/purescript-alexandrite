function eqInt$initialize$closure(argument0) {
  return argument1 => {
    const literal = true;
    return literal;
  };
}

function eqBoolean$initialize$closure(argument0) {
  return argument1 => {
    const literal = true;
    return literal;
  };
}

function eqArray$closure(argument0) {
  return argument1 => {
    const literal = true;
    return literal;
  };
}

function orderedInt$initialize$closure(argument0) {
  return argument1 => {
    const literal = true;
    return literal;
  };
}

export function eqArray(dictionary0) {
  const closure = eqArray$closure;
  const record = { eq: closure };
  return record;
}

export const eqInt = { eq: eqInt$initialize$closure };

export const eqBoolean = { eq: eqBoolean$initialize$closure };

export const orderedInt = { superclass62: eqInt, lessThanOrEqual: orderedInt$initialize$closure };
