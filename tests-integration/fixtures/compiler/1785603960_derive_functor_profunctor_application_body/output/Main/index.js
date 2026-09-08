import * as Data_Profunctor from "../Data.Profunctor/index.js";
export const Function = ($value0) => ({
  tag: "Function",
  _1: $value0
});
export const DoubleInput = ($value0) => ({
  tag: "DoubleInput",
  _1: $value0
});
export const profunctorFunction = /* @__PURE__ */ (() => {
  const $closure = (firstFunction) => {
    return (secondFunction) => {
      return (value) => {
        if (value.tag === "Function") {
          const { _1: field0 } = value;
          return {
            tag: "Function",
            _1: (argument) => secondFunction(field0(firstFunction(argument)))
          };
        }
        throw new Error("Pattern match failure");
      };
    };
  };
  return { dimap: $closure };
})();
const profunctorFunctionDictDimap = /* @__PURE__ */ Data_Profunctor.dimap(profunctorFunction);
export const functorDoubleInput = /* @__PURE__ */ (() => {
  const $closure = ($function) => {
    return (value) => {
      if (value.tag === "DoubleInput") {
        const { _1: field0 } = value;
        return {
          tag: "DoubleInput",
          _1: /* @__PURE__ */ profunctorFunctionDictDimap((element) => /* @__PURE__ */ profunctorFunctionDictDimap((element$1) => $function(element$1))((unchanged) => unchanged)(element))((unchanged$1) => unchanged$1)(field0)
        };
      }
      throw new Error("Pattern match failure");
    };
  };
  return { map: $closure };
})();
