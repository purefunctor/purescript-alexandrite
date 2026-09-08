export function choose(left) {
  return (right) => {
    return left;
  };
}
export function identity(value) {
  return value;
}
export const singleInfix = choose(1 | 0)("text");
export const multipleInfix = choose(choose(1 | 0)("first"))(true);
export const polymorphicHead = choose(identity)(1 | 0);
