export const Identity = ($value0) => ({
  tag: "Identity",
  _1: $value0
});
export function letBinding(value) {
  return value;
}
export function whereBinding(value) {
  return value;
}
export function siblingPolymorphicBindings(value) {
  return {
    first: ((inner) => inner)(value),
    second: ((inner$1) => inner$1)(value)
  };
}
export function unIdentity(value) {
  if (value.tag === "Identity") {
    const { _1: inner } = value;
    return inner;
  } else {
    throw new Error("Pattern match failure");
  }
}
