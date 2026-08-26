export const None = "None";
export const Some = ($value0) => ["Some", $value0];
export const Box = ($value0) => ["Box", $value0];
export function multiEquation($choice) {
  if ($choice === "None") {
    return 0 | 0;
  }
  if ($choice[0] === "Some") {
    const [, value] = $choice;
    return value;
  }
  throw new Error("Pattern match failure");
}
export function mixedArity($boolean) {
  return ($int) => {
    if ($boolean === true) {
      return ((value) => value)($int);
    }
    if ($boolean === false) {
      const value$1 = $int;
      return value$1;
    }
    throw new Error("Pattern match failure");
  };
}
export function singleConstructor($box) {
  if ($box[0] === "Box") {
    const [, value] = $box;
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
}
export function singleWildcards($int) {
  return ($int$1) => {
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
  return ($boolean) => {
    return captured;
  };
}
