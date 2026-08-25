export const Constructor = $value0 => ["Constructor", $value0];
export const Inl = $value0 => ["Inl", $value0];
export const Inr = $value0 => ["Inr", $value0];
export const Product = $value0 => $value1 => ["Product", $value0, $value1];
export const NoArguments = ["NoArguments"];

export function to(dictionary) {
  return dictionary.to;
}

export function from(dictionary) {
  return dictionary.from;
}
