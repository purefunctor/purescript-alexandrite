export const Pair = ($value0) => ($value1) => ({
  tag: "Pair",
  _1: $value0,
  _2: $value1
});
export function identity(value) {
  return value;
}
export const use = {
  tag: "Pair",
  _1: identity(42 | 0),
  _2: identity("x")
};
