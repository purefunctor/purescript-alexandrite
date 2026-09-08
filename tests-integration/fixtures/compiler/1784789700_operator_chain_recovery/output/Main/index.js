export function add(left) {
  return (right) => {
    return left;
  };
}
export const missingRightOperand = (() => {
  throw new Error("Generated code reached a source error");
})();
