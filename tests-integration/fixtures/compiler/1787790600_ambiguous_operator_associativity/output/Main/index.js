export function nonAssociative(left) {
  return (right) => {
    return left;
  };
}
export function leftAssociative(left) {
  return (right) => {
    return left;
  };
}
export function rightAssociative(left) {
  return (right) => {
    return right;
  };
}
export const singleNonAssociative = nonAssociative(1 | 0)(2 | 0);
export const ambiguousNonAssociative = (() => {
  throw new Error("Generated code reached a source error");
})();
export const validLeftAssociative = leftAssociative(leftAssociative(1 | 0)(2 | 0))(3 | 0);
export const validRightAssociative = rightAssociative(1 | 0)(rightAssociative(2 | 0)(3 | 0));
export const validMixedPrecedence = leftAssociative(1 | 0)(rightAssociative(2 | 0)(3 | 0));
export const ambiguousMixedAssociativity = (() => {
  throw new Error("Generated code reached a source error");
})();
