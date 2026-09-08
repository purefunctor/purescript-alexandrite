export function checked(value) {
  return value;
}
export function inferred(value) {
  return value;
}
export function multiple(first) {
  return (second) => {
    return first;
  };
}
