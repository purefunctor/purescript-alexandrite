export function make(dictionary) {
  return dictionary.make;
}
export function convert(dictionary) {
  return dictionary.convert;
}
export const makeRecordRecord = { make: (x) => x };
export const testMake = /* @__PURE__ */ make(makeRecordRecord);
export const convertRecordRecordRow = { convert: (x) => ({ converted: x }) };
export const testConvert = /* @__PURE__ */ convert(convertRecordRecordRow);
