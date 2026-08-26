export const None = "None";
export const Some = ($value0) => ({
  tag: "Some",
  _1: $value0
});
export const Box = ($value0) => ({
  tag: "Box",
  _1: $value0
});
export function multiEquation($choice) {
  if ($choice === "None") {
    return 0 | 0;
  }
  if ($choice.tag === "Some") {
    const { _1: value } = $choice;
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
  if ($box.tag === "Box") {
    const { _1: value } = $box;
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
