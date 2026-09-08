export function method(dictionary) {
  return dictionary.method;
}
export function test(myClassADict) {
  const $closure = (x) => {
    const baz = (myClassIntDict) => {
      return /* @__PURE__ */ method(myClassIntDict)(42 | 0);
    };
    return ((y) => /* @__PURE__ */ method(myClassADict)(y))(x);
  };
  return $closure;
}
export const myClassInt = { method: ($int) => 42 | 0 };
