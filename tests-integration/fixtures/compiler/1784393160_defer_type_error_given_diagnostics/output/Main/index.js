export function value(dictionary) {
  return dictionary.value;
}
export function guardedFailure(failTextDict) {
  return /* @__PURE__ */ ((failTextDict$1) => valueIntDictValue)(failTextDict);
}
export function guardedWarning(warnTextDict) {
  return /* @__PURE__ */ ((warnTextDict$1) => valueIntDictValue)(warnTextDict);
}
export const valueInt = { value: 42 | 0 };
const valueIntDictValue = /* @__PURE__ */ value(valueInt);
export const useFailure = (() => {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ guardedFailure($result);
})();
export const useWarning = /* @__PURE__ */ guardedWarning({});
