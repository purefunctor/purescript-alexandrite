export function clone(dictionary) {
  return dictionary.clone;
}
export function nest(dictionary) {
  return dictionary.nest;
}
export const cloneRecordRecord = { clone: (x) => x };
const cloneRecordRowRecordRowDictClone = /* @__PURE__ */ clone(cloneRecordRecord);
export const testClonePerson = cloneRecordRowRecordRowDictClone;
export const testCloneEmpty = cloneRecordRowRecordRowDictClone;
export const testCloneSingle = cloneRecordRowRecordRowDictClone;
export const nestRecordRecordRow = { nest: (x) => ({
  inner: x,
  outer: 0 | 0
}) };
export const testNest = /* @__PURE__ */ nest(nestRecordRecordRow);
