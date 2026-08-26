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
    return "Empty";
  }
  if ($choice[0] === "One") {
    const [, value] = $choice;
    return ["One", value];
  }
  if ($choice[0] === "Pair") {
    const whole = $choice;
    const [, left] = $choice;
    if (whole[0] === "Pair") {
      return ["One", left];
    }
    return "Empty";
  }
  throw new Error("Pattern match failure");
}
export function pair($choice) {
  if ($choice[0] === "Pair") {
    const [, left, right] = $choice;
    return [
      "Pair",
      left,
      right
    ];
  }
  const choice = $choice;
  return choice;
}
export function unwrap(value) {
  return value;
}
export function nested($nested) {
  if ($nested[0] === "Outer" && $nested[1][0] === "One") {
    const [, value] = $nested[1];
    return ["One", value];
  }
  return "Empty";
}
export function bind(value) {
  return (continuation) => {
    return continuation(value);
  };
}
export function ordinaryBind(identity) {
  return bind(identity)((value) => value);
}
