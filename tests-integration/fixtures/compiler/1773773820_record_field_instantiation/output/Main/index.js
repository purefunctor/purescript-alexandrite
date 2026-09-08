export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function constructor(state) {
  return {
    ...state,
    example: "Nothing"
  };
}
export function variable(state) {
  return {
    ...state,
    example: "Nothing"
  };
}
