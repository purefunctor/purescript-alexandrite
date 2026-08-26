import * as $foreign from "./foreign.js";
export function add(dictionary) {
  return dictionary.add;
}
export function mul(dictionary) {
  return dictionary.mul;
}
export const intAdd = $foreign["intAdd"];
export const intMultiply = $foreign["intMultiply"];
export const semiringInt = {
  add: intAdd,
  mul: intMultiply
};
