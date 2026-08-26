export function apply($function) {
  return (value) => {
    return $function(value);
  };
}
export function capture(captured) {
  return ($int) => captured;
}
export function choose(condition) {
  return (left) => {
    return (right) => {
      if (condition) {
        return left;
      } else {
        return right;
      }
    };
  };
}
export function literalCase(value) {
  if (value === (0 | 0)) {
    return "zero";
  }
  return "other";
}
export const partial = choose(true)(42 | 0);
export const higherOrder = apply(capture(42 | 0))(0 | 0);
