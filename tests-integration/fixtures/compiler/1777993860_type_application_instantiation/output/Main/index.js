export const Left = ($value0) => ({
  tag: "Left",
  _1: $value0
});
export const Right = ($value0) => ({
  tag: "Right",
  _1: $value0
});
function $const(a) {
  return ($b) => {
    return a;
  };
}
export const forceSolve = {
  const: $const,
  left: Left,
  right: Right
};
export { $const as "const" };
