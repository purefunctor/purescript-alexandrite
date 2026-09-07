import * as Data_Foldable from "../Data.Foldable/index.js";
import * as Data_Functor from "../Data.Functor/index.js";
import * as Data_Traversable from "../Data.Traversable/index.js";
export const Compose = ($value0) => ({
  tag: "Compose",
  _1: $value0
});
export function functorComposeTypeType(functorFDict) {
  return (functorGDict) => {
    const $closure = ($function) => {
      return (value) => {
        if (value.tag === "Compose") {
          const { _1: field0 } = value;
          return {
            tag: "Compose",
            _1: /* @__PURE__ */ Data_Functor.map(functorFDict)((element) => /* @__PURE__ */ Data_Functor.map(functorGDict)((element$1) => $function(element$1))(element))(field0)
          };
        }
        throw new Error("Pattern match failure");
      };
    };
    return { map: $closure };
  };
}
export function foldableComposeTypeType(foldableFDict) {
  return (foldableGDict) => {
    const $closure = ($function) => {
      return (accumulator) => {
        return (value) => {
          if (value.tag === "Compose") {
            const { _1: field0 } = value;
            return /* @__PURE__ */ Data_Foldable.foldr(foldableFDict)((element) => (accumulator$1) => /* @__PURE__ */ Data_Foldable.foldr(foldableGDict)((element$1) => (accumulator$2) => $function(element$1)(accumulator$2))(accumulator$1)(element))(accumulator)(field0);
          }
          throw new Error("Pattern match failure");
        };
      };
    };
    const $closure$1 = ($function$1) => {
      return (accumulator$3) => {
        return (value$1) => {
          if (value$1.tag === "Compose") {
            const { _1: field0$1 } = value$1;
            return /* @__PURE__ */ Data_Foldable.foldl(foldableFDict)((accumulator$4) => (element$2) => /* @__PURE__ */ Data_Foldable.foldl(foldableGDict)((accumulator$5) => (element$3) => $function$1(accumulator$5)(element$3))(accumulator$4)(element$2))(accumulator$3)(field0$1);
          }
          throw new Error("Pattern match failure");
        };
      };
    };
    const $closure$2 = (monoidMDict) => {
      const $closure$3 = ($function$2) => {
        return (value$2) => {
          if (value$2.tag === "Compose") {
            const { _1: field0$2 } = value$2;
            return /* @__PURE__ */ Data_Foldable.foldMap(foldableFDict)(monoidMDict)((element$4) => /* @__PURE__ */ Data_Foldable.foldMap(foldableGDict)(monoidMDict)((element$5) => $function$2(element$5))(element$4))(field0$2);
          }
          throw new Error("Pattern match failure");
        };
      };
      return $closure$3;
    };
    return {
      foldr: $closure,
      foldl: $closure$1,
      foldMap: $closure$2
    };
  };
}
export function traversableComposeTypeType(traversableFDict) {
  return (traversableGDict) => {
    const $closure = (applicativeMDict) => {
      const $closure$1 = ($function) => {
        return (value) => {
          if (value.tag === "Compose") {
            const { _1: field0 } = value;
            return /* @__PURE__ */ Data_Functor.map(/* @__PURE__ */ (/* @__PURE__ */ applicativeMDict.Apply0()).Functor0())((field0Result) => ({
              tag: "Compose",
              _1: field0Result
            }))(/* @__PURE__ */ Data_Traversable.traverse(traversableFDict)(applicativeMDict)((element) => /* @__PURE__ */ Data_Traversable.traverse(traversableGDict)(applicativeMDict)((element$1) => $function(element$1))(element))(field0));
          }
          throw new Error("Pattern match failure");
        };
      };
      return $closure$1;
    };
    return {
      Functor0: () => /* @__PURE__ */ functorComposeTypeType(/* @__PURE__ */ traversableFDict.Functor0())(/* @__PURE__ */ traversableGDict.Functor0()),
      Foldable1: () => /* @__PURE__ */ foldableComposeTypeType(/* @__PURE__ */ traversableFDict.Foldable1())(/* @__PURE__ */ traversableGDict.Foldable1()),
      traverse: $closure,
      sequence: (applicativeMDict$1) => (value$1) => /* @__PURE__ */ Data_Traversable.traverse(/* @__PURE__ */ traversableComposeTypeType(traversableFDict)(traversableGDict))(applicativeMDict$1)((effect0) => effect0)(value$1)
    };
  };
}
