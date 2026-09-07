export const Left = ($value0) => ({
  tag: "Left",
  _1: $value0
});
export const Right = ($value0) => ({
  tag: "Right",
  _1: $value0
});
export function isLeft($either) {
  if ($either.tag === "Left") {
    return true;
  }
  if ($either.tag === "Right") {
    return false;
  }
  throw new Error("Pattern match failure");
}
export function isRight($either) {
  if ($either.tag === "Left") {
    return false;
  }
  if ($either.tag === "Right") {
    return true;
  }
  throw new Error("Pattern match failure");
}
export const test = {
  left: Left,
  right: Right,
  isLeft,
  isRight
};
