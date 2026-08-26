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
      if (tree.tag === "Leaf") {
        const { _1: value } = tree;
        if (stack === "Empty") {
          return addInt(accumulator)(value);
        }
        if (stack.tag === "Push") {
          const { _1: next, _2: rest } = stack;
          return walkTree(next)(rest)(addInt(accumulator)(value));
        }
        throw new Error("Pattern match failure");
      }
      if (tree.tag === "Branch") {
        const { _1: left, _2: right } = tree;
        return walkTree(left)({
          tag: "Push",
          _1: right,
          _2: stack
        })(accumulator);
      }
      throw new Error("Pattern match failure");
    };
  };
}
export const addInt = $foreign["addInt"];
