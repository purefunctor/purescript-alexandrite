import * as $foreign from "./foreign.js";
export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const Node = ($value0) => ($value1) => ($value2) => ({
  tag: "Node",
  _1: $value0,
  _2: $value1,
  _3: $value2
});
export const force = $foreign["force"];
