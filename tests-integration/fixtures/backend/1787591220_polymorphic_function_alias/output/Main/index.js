export const Pair = ($value0) => ($value1) => [
  "Pair",
  $value0,
  $value1
];
export function identity(value) {
  return value;
}
export const use = [
  "Pair",
  identity(42 | 0),
  identity("x")
];
