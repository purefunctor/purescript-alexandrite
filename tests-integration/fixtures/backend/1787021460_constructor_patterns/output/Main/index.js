export const Empty = "Empty";
export const One = ($value0) => ["One", $value0];
export const Pair = ($value0) => ($value1) => [
  "Pair",
  $value0,
  $value1
];
export const Outer = ($value0) => ["Outer", $value0];
export function first($choice) {
  if ($choice === "Empty") {
    return Empty;
  }
  if (Array.isArray($choice) && $choice[0] === "One") {
    const value = $choice[1];
    return One(value);
  }
  if (Array.isArray($choice) && $choice[0] === "Pair") {
    const whole = $choice;
    const left = $choice[1];
    if (Array.isArray(whole) && whole[0] === "Pair") {
      return One(left);
    }
    return Empty;
  }
  throw new Error("Pattern match failure");
}
export function unwrap(value) {
  return value;
}
export function nested($nested) {
  if (Array.isArray($nested) && $nested[0] === "Outer" && Array.isArray($nested[1]) && $nested[1][0] === "One") {
    const value = $nested[1][1];
    return One(value);
  }
  return Empty;
}
export function bind(value) {
  return (continuation) => {
    return continuation(value);
  };
}
export function ordinaryBind(identity) {
  return bind(identity)((value) => value);
}
