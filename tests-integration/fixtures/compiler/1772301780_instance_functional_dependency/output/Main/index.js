export function convert(dictionary) {
  return dictionary.convert;
}
export const convertIntString = { convert: ($int) => "int" };
export const test = /* @__PURE__ */ convert(convertIntString)(42 | 0);
export const typeEq = {};
