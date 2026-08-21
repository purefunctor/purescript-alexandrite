export const None = ["None"];
export const Some = $value0 => ["Some", $value0];
export const Box = $value0 => ["Box", $value0];

export function multiEquation(argument0) {
  if (Array.isArray(argument0) && argument0[0] === "None") {
    return 0 | 0;
  } else {
    if (Array.isArray(argument0) && argument0[0] === "Some") {
      return argument0[1];
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function mixedArity(argument0) {
  return argument1 => {
    if (argument0 === true) {
      function mixedArity$closure(value) {
        return value;
      }
      return mixedArity$closure(argument1);
    } else {
      if (argument0 === false) {
        return argument1;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

export function singleConstructor(argument0) {
  if (Array.isArray(argument0) && argument0[0] === "Box") {
    return argument0[1];
  } else {
    throw new Error("Pattern match failure");
  }
}

export function singleWildcards(argument0) {
  return argument1 => {
    return true;
  };
}

export function functionWildcard(argument0) {
  return 0 | 0;
}

export function rigidWildcard(argument0) {
  return true;
}

export function namedPattern(record) {
  const value = record.value;
  return record;
}

export function capture(captured) {
  return argument1 => {
    return captured;
  };
}
