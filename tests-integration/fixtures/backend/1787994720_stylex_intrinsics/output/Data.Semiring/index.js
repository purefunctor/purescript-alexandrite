import * as $foreign from "./foreign.js";
export function add(dictionary) {
  return dictionary.add;
}
export function zero(dictionary) {
  return dictionary.zero;
}
export function mul(dictionary) {
  return dictionary.mul;
}
export function one(dictionary) {
  return dictionary.one;
}
export const intAdd = $foreign["intAdd"];
export const intMultiply = $foreign["intMultiply"];
export const numAdd = $foreign["numAdd"];
export const numMul = $foreign["numMul"];
export const semiringInt = {
  add: intAdd,
  zero: 0 | 0,
  mul: intMultiply,
  one: 1 | 0
};
export const semiringNumber = {
  add: numAdd,
  zero: 0,
  mul: numMul,
  one: 1
};
