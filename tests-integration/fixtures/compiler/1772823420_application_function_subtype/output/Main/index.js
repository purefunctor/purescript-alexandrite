export function identity(dictionary) {
  return dictionary.identity;
}
export const categoryFn = { identity: (x) => x };
const categoryFunctionDictIdentity = /* @__PURE__ */ identity(categoryFn);
export const test = categoryFunctionDictIdentity;
const test_ = categoryFunctionDictIdentity;
export { test_ as "test'" };
