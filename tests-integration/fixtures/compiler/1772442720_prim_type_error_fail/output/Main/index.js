export const Proxy = "Proxy";
export function failBasic(failTextDict) {
  return (x) => x;
}
export function failComplex(failAboveTextBesideTextDict) {
  return (p) => p;
}
export const useFailBasic = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ failBasic($result)(42 | 0);
})();
export const useFailComplex = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ failComplex($result)("Proxy");
})();
