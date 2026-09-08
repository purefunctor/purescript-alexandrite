export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export const MkId = ($value0) => ({
  tag: "MkId",
  _1: $value0
});
export function test(partialDict) {
  if (identity.tag === "Just") {
    const { _1: f } = identity;
    const $scrutinee = f(42 | 0);
    return f(true);
  }
  throw new Error("Pattern match failure");
}
export function test2(x) {
  if (x.tag === "MkId") {
    const { _1: f } = x;
    const $scrutinee = f(42 | 0);
    return f(true);
  }
  throw new Error("Pattern match failure");
}
export function test3(partialDict) {
  if (identity.tag === "Just") {
    const { _1: f } = identity;
    const $scrutinee = f(42 | 0);
    return f(true);
  } else {
    throw new Error("Pattern match failure");
  }
}
export function test4(x) {
  if (x.tag === "MkId") {
    const { _1: f } = x;
    const $scrutinee = f(42 | 0);
    return f(true);
  } else {
    throw new Error("Pattern match failure");
  }
}
export const identity = "Nothing";
