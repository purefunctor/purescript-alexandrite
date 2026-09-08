export function convert(dictionary) {
  return dictionary.convert;
}
export const convertInt = 0 | 0;
export const convertInt1 = { convert: (value) => value };
export const test = /* @__PURE__ */ convert(convertInt1)(42 | 0);
