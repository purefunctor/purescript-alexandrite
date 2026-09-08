import * as $foreign from "./foreign.js";
export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const pure = $foreign["pure"];
export const test = bind(pure(1 | 0))((x) => bind(pure("hello"))((y) => pure({
  tag: "Tuple",
  _1: x,
  _2: y
})));
const test_ = bind(pure(1 | 0))((x) => bind(pure("hello"))((y) => pure({
  tag: "Tuple",
  _1: x,
  _2: y
})));
export { test_ as "test'" };
