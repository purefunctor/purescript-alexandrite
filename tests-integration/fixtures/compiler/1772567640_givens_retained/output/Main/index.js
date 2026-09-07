export function consume(dictionary) {
  return dictionary.consume;
}
export function testGiven(givenADict) {
  const $closure = (a) => {
    const b = /* @__PURE__ */ consume(givenADict)(a);
    const c = /* @__PURE__ */ consume(givenADict)(a);
    return /* @__PURE__ */ consume(givenADict)(a);
  };
  return $closure;
}
