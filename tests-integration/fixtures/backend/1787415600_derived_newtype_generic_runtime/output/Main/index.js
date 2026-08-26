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
  return Data_Generic_Rep.to(genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt)(Data_Generic_Rep.from(genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt)(value));
}
const $lazy_genericVoidNoConstructors = $runtime.binding("genericVoidNoConstructors", () => {
  const $closure = (value) => {
    return Data_Generic_Rep.to($lazy_genericVoidNoConstructors())(value);
  };
  const $closure$1 = (value$1) => {
    return Data_Generic_Rep.from($lazy_genericVoidNoConstructors())(value$1);
  };
  return {
    to: $closure,
    from: $closure$1
  };
});
export const newtypeTypeIdentifierInt = { Coercible0: () => ({}) };
export const wrapped = 42 | 0;
export const unwrapped = Data_Newtype.unwrap(newtypeTypeIdentifierInt)(wrapped);
export const genericChoiceSumConstructorNoArgumentsSumConstructorArgumentIntConstructorProductArgumentIntArgumentInt = (() => {
  const $closure = (representation) => {
    if (Array.isArray(representation) && representation[0] === "Inl" && Array.isArray(representation[1]) && representation[1][0] === "Constructor" && representation[1][1] === "NoArguments") {
      return "Empty";
    }
    if (Array.isArray(representation) && representation[0] === "Inr" && Array.isArray(representation[1]) && representation[1][0] === "Inl" && Array.isArray(representation[1][1]) && representation[1][1][0] === "Constructor") {
      const field0 = representation[1][1][1];
      return ["Single", field0];
    }
    if (Array.isArray(representation) && representation[0] === "Inr" && Array.isArray(representation[1]) && representation[1][0] === "Inr" && Array.isArray(representation[1][1]) && representation[1][1][0] === "Constructor" && Array.isArray(representation[1][1][1]) && representation[1][1][1][0] === "Product") {
      const field0$1 = representation[1][1][1][1];
      const field1 = representation[1][1][1][2];
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
    if (Array.isArray(value) && value[0] === "Single") {
      const field0$2 = value[1];
      return ["Inr", ["Inl", ["Constructor", field0$2]]];
    }
    if (Array.isArray(value) && value[0] === "Pair") {
      const field0$3 = value[1];
      const field1$1 = value[2];
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
