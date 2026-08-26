export const Constructor = ($value0) => ({
  tag: "Constructor",
  _1: $value0
});
export const Inl = ($value0) => ({
  tag: "Inl",
  _1: $value0
});
export const Inr = ($value0) => ({
  tag: "Inr",
  _1: $value0
});
export const Product = ($value0) => ($value1) => ({
  tag: "Product",
  _1: $value0,
  _2: $value1
});
export const NoArguments = "NoArguments";
export function to(dictionary) {
  return dictionary.to;
}
export function from(dictionary) {
  return dictionary.from;
}
