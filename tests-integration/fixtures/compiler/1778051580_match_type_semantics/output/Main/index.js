export const Unit = "Unit";
export const Output = "Output";
export function isEq(dictionary) {
  return dictionary.isEq;
}
export function testSkolemRight(x) {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ isEq($result)(x)("Unit");
}
export function testSkolemLeft(x) {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ isEq($result)("Unit")(x);
}
export function testSkolemIsEq(x) {
  return (y) => {
    let $result;
    throw new Error("Generated code reached a source error");
    return /* @__PURE__ */ isEq($result)(y)(x);
  };
}
export function typeEquals(typeEqualsABDict) {
  return "Unit";
}
export function rowSame(dictionary) {
  return dictionary.rowSame;
}
export const isEqOutput = { isEq: ($a) => ($a$1) => "Output" };
export const isEqOutput1 = { isEq: ($a) => ($b) => "Output" };
export const testEqConcrete = /* @__PURE__ */ isEq(isEqOutput)("Unit")("Unit");
export const typeEquals1 = {};
export const testOpenRows = /* @__PURE__ */ typeEquals(typeEquals1);
export const rowSame1 = { rowSame: "Unit" };
export const testOpenRowsNoFd = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ rowSame($result);
})();
