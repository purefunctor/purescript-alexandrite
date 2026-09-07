export const Pair = ($value0) => ($value1) => ({
  tag: "Pair",
  _1: $value0,
  _2: $value1
});
export function convert(dictionary) {
  return dictionary.convert;
}
export const convertIntString = { convert: ($int) => "" };
export const convertPairIntStringBoolean = { convert: ($pair) => true };
export const convert1 = { convert: (value) => value };
export const convertPair = { convert: (value) => ({
  tag: "Pair",
  _1: value,
  _2: value
}) };
