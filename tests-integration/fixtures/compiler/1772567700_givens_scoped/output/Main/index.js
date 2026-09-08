export function consume(dictionary) {
  return dictionary.consume;
}
export function testGiven(givenADict) {
  const $closure = (a) => {
    const b = (givenIntDict) => {
      const consumeInt = /* @__PURE__ */ consume(givenIntDict)(42 | 0);
      return a;
    };
    let $result;
    throw new Error("Generated code reached a source error");
    const c = /* @__PURE__ */ consume($result)(42 | 0);
    return /* @__PURE__ */ consume(givenADict)(a);
  };
  return $closure;
}
