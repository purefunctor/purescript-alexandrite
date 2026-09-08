export function test(dictionary) {
  return dictionary.test;
}
export const testRefl = { test: (x) => x };
export const testInt = { test: ($int) => 0 | 0 };
