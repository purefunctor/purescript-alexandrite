export const Pair = ($value0) => ($value1) => ({
  tag: "Pair",
  _1: $value0,
  _2: $value1
});
export function test(dictionary) {
  return dictionary.test;
}
export const testLeft = { test: ($pair) => 0 | 0 };
export const testRight = { test: ($pair) => 1 | 0 };
