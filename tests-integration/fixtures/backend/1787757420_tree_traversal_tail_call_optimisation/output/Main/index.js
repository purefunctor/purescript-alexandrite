import * as $foreign from "./foreign.js";
export const Leaf = ($value0) => ({
  tag: "Leaf",
  _1: $value0
});
export const Branch = ($value0) => ($value1) => ({
  tag: "Branch",
  _1: $value0,
  _2: $value1
});
export const Empty = "Empty";
export const Push = ($value0) => ($value1) => ({
  tag: "Push",
  _1: $value0,
  _2: $value1
});
export function sumTree(tree) {
  return walkTree(tree)("Empty")(0 | 0);
}
export function walkTree(tree) {
  return (stack) => {
    return (accumulator) => {
      let $argument0 = tree;
      let $argument1 = stack;
      let $argument2 = accumulator;
      while (true) {
        const $currentArgument0 = $argument0;
        const $currentArgument1 = $argument1;
        const $currentArgument2 = $argument2;
        if ($currentArgument0.tag === "Leaf") {
          const { _1: value } = $currentArgument0;
          if ($currentArgument1 === "Empty") {
            return addInt($currentArgument2)(value);
          }
          if ($currentArgument1.tag === "Push") {
            const { _1: next, _2: rest } = $currentArgument1;
            $argument0 = next;
            $argument1 = rest;
            $argument2 = addInt($currentArgument2)(value);
            continue;
          }
          throw new Error("Pattern match failure");
        }
        if ($currentArgument0.tag === "Branch") {
          const { _1: left, _2: right } = $currentArgument0;
          $argument0 = left;
          $argument1 = {
            tag: "Push",
            _1: right,
            _2: $currentArgument1
          };
          $argument2 = $currentArgument2;
          continue;
        }
        throw new Error("Pattern match failure");
      }
    };
  };
}
export const addInt = $foreign["addInt"];
