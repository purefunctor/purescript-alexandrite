export const None = ["None"];
export const Some = $value0 => ["Some", $value0];
export const Box = $value0 => ["Box", $value0];

export function multiEquation($choice) {
  if (Array.isArray($choice) && $choice[0] === "None") {
    return 0 | 0;
  } else {
    if (Array.isArray($choice) && $choice[0] === "Some") {
      return $choice[1];
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function mixedArity($boolean) {
  return $int => {
    if ($boolean === true) {
      return (value => value)($int);
    } else {
      if ($boolean === false) {
        return $int;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

export function singleConstructor($box) {
  if (Array.isArray($box) && $box[0] === "Box") {
    return $box[1];
  } else {
    throw new Error("Pattern match failure");
  }
}

export function singleWildcards($int) {
  return $int$1 => {
    return true;
  };
}

export function functionWildcard($function) {
  return 0 | 0;
}

export function rigidWildcard($value) {
  return true;
}

export function namedPattern(record) {
  const value = record.value;
  return record;
}

export function capture(captured) {
  return $boolean => {
    return captured;
  };
}
