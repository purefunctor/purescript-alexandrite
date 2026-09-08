import * as $foreign from "./foreign.js";
export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const pure = $foreign["pure"];
export const test = apply(map((x) => (y) => ({
  tag: "Tuple",
  _1: x,
  _2: y
}))(pure(1 | 0)))(pure("hello"));
const test_ = apply(map((x) => (y) => ({
  tag: "Tuple",
  _1: x,
  _2: y
}))(pure(1 | 0)))(pure("hello"));
export { test_ as "test'" };
