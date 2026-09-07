export function map(dictionary) {
  return dictionary.map;
}
export const functorArray = { map: ($function) => ($array) => [] };
export const testMap = /* @__PURE__ */ map(functorArray);
