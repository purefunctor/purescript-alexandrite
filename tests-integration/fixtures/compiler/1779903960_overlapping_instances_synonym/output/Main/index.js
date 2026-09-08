export function convert(dictionary) {
  return dictionary.convert;
}
export const convertSB = { convert: (s) => s };
export const convertSS = { convert: (s) => s };
export const value = /* @__PURE__ */ convert(convertSB)("value");
