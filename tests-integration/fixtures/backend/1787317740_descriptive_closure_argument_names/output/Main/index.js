export const None = ["None"];
export const Some = $value0 => ["Some", $value0];
export const Box = $value0 => ["Box", $value0];

export function multiEquation($choice) {
  if (Array.isArray($choice) && $choice[0] === "None") {
    return 0 | 0;
  }
  if (Array.isArray($choice) && $choice[0] === "Some") {
    const value = $choice[1];
    return value;
  }
  throw new Error("Pattern match failure");
}

export function mixedArity($boolean) {
  return $int => {
    if ($boolean === true) {
      return (value => value)($int);
    }
    if ($boolean === false) {
      const value$1 = $int;
      return value$1;
    }
    throw new Error("Pattern match failure");
  };
}

export function singleConstructor($box) {
  if (Array.isArray($box) && $box[0] === "Box") {
    const value = $box[1];
    return value;
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
  return record;
}

export function capture(captured) {
  return $boolean => {
    return captured;
  };
}
