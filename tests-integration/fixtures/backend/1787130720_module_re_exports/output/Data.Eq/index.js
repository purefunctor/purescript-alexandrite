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

function eqRec$closure(argument0) {
  return argument1 => {
    const literal = true;
    return literal;
  };
}

function eqRecordNilType$initialize$closure(argument0) {
  return argument1 => {
    return argument2 => {
      const literal = true;
      return literal;
    };
  };
}

function eqRecordConsType$closure(argument0) {
  return argument1 => {
    return argument2 => {
      const literal = true;
      return literal;
    };
  };
}

export function eqRec(dictionary0) {
  return dictionary1 => {
    const closure = eqRec$closure;
    const record = { eq: closure };
    return record;
  };
}

export function eqRecordConsType(dictionary2) {
  return dictionary3 => {
    return dictionary4 => {
      return dictionary5 => {
        const closure = eqRecordConsType$closure;
        const record = { eqRecord: closure };
        return record;
      };
    };
  };
}

export const eqInt = { eq: eqInt$initialize$closure };

export const eqBoolean = { eq: eqBoolean$initialize$closure };

export const eqRecordNilType = { eqRecord: eqRecordNilType$initialize$closure };
