import * as $foreign from "./foreign.js";
export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const pure = $foreign["pure"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const zero = pure(1 | 0);
export const zeroWithLet = pure(1 | 0);
export const one = map((value) => value)(pure(1 | 0));
export const multiple = apply(map((first) => (second) => ({
  tag: "Tuple",
  _1: first,
  _2: second
}))(pure(1 | 0)))(pure("two"));
export const discarded = apply(map(($int) => (value) => value)(pure(1 | 0)))(pure("kept"));
export const withLet = apply(map((first) => (second) => ({
  tag: "Tuple",
  _1: first,
  _2: second
}))(pure(1 | 0)))(pure(2 | 0));
export const nested = map((value) => value)(map((inner) => inner)(pure(1 | 0)));
