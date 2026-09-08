export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export function map(dictionary) {
  return dictionary.map;
}
export function apply(dictionary) {
  return dictionary.apply;
}
export function pure(dictionary) {
  return dictionary.pure;
}
export function discard(dictionary) {
  return dictionary.discard;
}
export function bind(dictionary) {
  return dictionary.bind;
}
export function testDo(monadMDict) {
  const Bind0Dict = /* @__PURE__ */ monadMDict.Bind0();
  const $closure = (x) => {
    const Bind0Dict$1 = /* @__PURE__ */ monadMDict.Bind0();
    return /* @__PURE__ */ bind(Bind0Dict$1)(/* @__PURE__ */ pure(/* @__PURE__ */ Bind0Dict$1.Applicative0())("hello"))((y) => /* @__PURE__ */ pure(/* @__PURE__ */ (/* @__PURE__ */ monadMDict.Bind0()).Applicative0())({
      tag: "Tuple",
      _1: x,
      _2: y
    }));
  };
  return /* @__PURE__ */ bind(Bind0Dict)(/* @__PURE__ */ pure(/* @__PURE__ */ Bind0Dict.Applicative0())(1 | 0))($closure);
}
function testDo_(bindDict) {
  return /* @__PURE__ */ bind(bindDict)(/* @__PURE__ */ pure(/* @__PURE__ */ bindDict.Applicative0())(1 | 0))((x) => /* @__PURE__ */ bind(bindDict)(/* @__PURE__ */ pure(/* @__PURE__ */ bindDict.Applicative0())("hello"))((y) => /* @__PURE__ */ pure(/* @__PURE__ */ bindDict.Applicative0())({
    tag: "Tuple",
    _1: x,
    _2: y
  })));
}
export function testAdo(applicativeFDict) {
  const Apply0Dict = /* @__PURE__ */ applicativeFDict.Apply0();
  return /* @__PURE__ */ apply(Apply0Dict)(/* @__PURE__ */ map(/* @__PURE__ */ Apply0Dict.Functor0())((x) => (y) => ({
    tag: "Tuple",
    _1: x,
    _2: y
  }))(/* @__PURE__ */ pure(applicativeFDict)(1 | 0)))(/* @__PURE__ */ pure(applicativeFDict)("hello"));
}
function testAdo_(applicativeDict) {
  const Apply0Dict = /* @__PURE__ */ applicativeDict.Apply0();
  return /* @__PURE__ */ apply(Apply0Dict)(/* @__PURE__ */ map(/* @__PURE__ */ Apply0Dict.Functor0())((x) => (y) => ({
    tag: "Tuple",
    _1: x,
    _2: y
  }))(/* @__PURE__ */ pure(applicativeDict)(1 | 0)))(/* @__PURE__ */ pure(applicativeDict)("hello"));
}
export function testDoDiscard(monadMDict) {
  let $result;
  throw new Error("Generated code reached a source error");
  return /* @__PURE__ */ discard($result)(/* @__PURE__ */ pure(/* @__PURE__ */ (/* @__PURE__ */ monadMDict.Bind0()).Applicative0())("ignored"))(($string) => /* @__PURE__ */ pure(/* @__PURE__ */ (/* @__PURE__ */ monadMDict.Bind0()).Applicative0())(42 | 0));
}
function testDoDiscard_(discardDict) {
  return /* @__PURE__ */ discard(discardDict)(/* @__PURE__ */ pure(/* @__PURE__ */ discardDict.Applicative0())("ignored"))(($string) => /* @__PURE__ */ pure(/* @__PURE__ */ discardDict.Applicative0())(42 | 0));
}
export { testDo_ as "testDo'" };
export { testAdo_ as "testAdo'" };
export { testDoDiscard_ as "testDoDiscard'" };
