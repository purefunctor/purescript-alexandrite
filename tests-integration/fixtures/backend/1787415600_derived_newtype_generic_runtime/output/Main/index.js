import * as Data_Generic_Rep from "../Data.Generic.Rep/index.js";
import * as Data_Newtype from "../Data.Newtype/index.js";
import * as $runtime from "../runtime.js";
export const Empty = "Empty";
export const Single = ($value0) => ["Single", $value0];
export const Pair = ($value0) => ($value1) => [
  "Pair",
  $value0,
  $value1
];
export function roundTrip(value) {
  return /* @__PURE__ */ Data_Generic_Rep.to(genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt)(/* @__PURE__ */ Data_Generic_Rep.from(genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt)(value));
}
const $lazy_genericVoidNoConstructors = $runtime.binding("genericVoidNoConstructors", () => {
  const $closure = (value) => {
    return /* @__PURE__ */ Data_Generic_Rep.to($lazy_genericVoidNoConstructors())(value);
  };
  const $closure$1 = (value$1) => {
    return /* @__PURE__ */ Data_Generic_Rep.from($lazy_genericVoidNoConstructors())(value$1);
  };
  return {
    to: $closure,
    from: $closure$1
  };
});
export const newtypeTypeIdentifierInt = { Coercible0: () => ({}) };
export const wrapped = 42 | 0;
export const unwrapped = /* @__PURE__ */ Data_Newtype.unwrap(newtypeTypeIdentifierInt)(wrapped);
export const genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt = /* @__PURE__ */ (() => {
  const $closure = (representation) => {
    if (representation[0] === "Inl" && representation[1][0] === "Constructor" && representation[1][1] === "NoArguments") {
      return "Empty";
    }
    if (representation[0] === "Inr" && representation[1][0] === "Inl" && representation[1][1][0] === "Constructor") {
      const [, field0] = representation[1][1];
      return ["Single", field0];
    }
    if (representation[0] === "Inr" && representation[1][0] === "Inr" && representation[1][1][0] === "Constructor" && representation[1][1][1][0] === "Product") {
      const [, field0$1, field1] = representation[1][1][1];
      return [
        "Pair",
        field0$1,
        field1
      ];
    }
    throw new Error("Pattern match failure");
  };
  const $closure$1 = (value) => {
    if (value === "Empty") {
      return ["Inl", ["Constructor", "NoArguments"]];
    }
    if (value[0] === "Single") {
      const [, field0$2] = value;
      return ["Inr", ["Inl", ["Constructor", field0$2]]];
    }
    if (value[0] === "Pair") {
      const [, field0$3, field1$1] = value;
      return ["Inr", ["Inr", ["Constructor", [
        "Product",
        field0$3,
        field1$1
      ]]]];
    }
    throw new Error("Pattern match failure");
  };
  return {
    to: $closure,
    from: $closure$1
  };
})();
export const emptyRoundTrip = roundTrip("Empty");
export const singleRoundTrip = roundTrip(["Single", 6 | 0]);
export const pairRoundTrip = roundTrip([
  "Pair",
  7 | 0,
  8 | 0
]);
export const genericVoidNoConstructors = $lazy_genericVoidNoConstructors();
