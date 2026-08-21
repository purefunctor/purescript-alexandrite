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

function eqRec$closure(argument0) {
  return argument1 => {
    return true;
  };
}

function eqRecordNilType$initialize$closure(argument0) {
  return argument1 => {
    return argument2 => {
      return true;
    };
  };
}

function eqRecordConsType$closure(argument0) {
  return argument1 => {
    return argument2 => {
      return true;
    };
  };
}

export function eqRec(dictionary0) {
  return dictionary1 => {
    const record = { eq: eqRec$closure };
    return record;
  };
}

export function eqRecordConsType(dictionary2) {
  return dictionary3 => {
    return dictionary4 => {
      return dictionary5 => {
        const record = { eqRecord: eqRecordConsType$closure };
        return record;
      };
    };
  };
}

export const eqInt = { eq: eqInt$initialize$closure };

export const eqBoolean = { eq: eqBoolean$initialize$closure };

export const eqRecordNilType = { eqRecord: eqRecordNilType$initialize$closure };
