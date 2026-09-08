export function disj(dictionary) {
  return dictionary.disj;
}
export function heytingAlgebraFunction(heytingAlgebraBDict) {
  return { disj: (left) => (right) => (value) => /* @__PURE__ */ disj(heytingAlgebraBDict)(left(value))(right(value)) };
}
export const heytingAlgebraBoolean = { disj: (left) => ($boolean) => left };
export const test = /* @__PURE__ */ disj(/* @__PURE__ */ heytingAlgebraFunction(heytingAlgebraBoolean));
