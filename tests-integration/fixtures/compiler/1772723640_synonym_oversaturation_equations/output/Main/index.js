export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const test1 = [42 | 0];
export const test2 = {
  tag: "Tuple",
  _1: 42 | 0,
  _2: "hello"
};
export const forceSolve = {
  test1,
  test2
};
