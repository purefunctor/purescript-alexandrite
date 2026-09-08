export function test(dictionary) {
  return dictionary.test;
}
export const testInt = { test: ($int) => 0 | 0 };
export const testRefl = { test: (x) => x };
export const value = /* @__PURE__ */ test(testInt)(1 | 0);
