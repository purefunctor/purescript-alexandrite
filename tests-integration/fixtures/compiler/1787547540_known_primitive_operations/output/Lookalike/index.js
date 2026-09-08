import * as $foreign from "./foreign.js";
export function add(dictionary) {
  return dictionary.add;
}
export function negate(value) {
  return value;
}
export const intAdd = $foreign["intAdd"];
export const semiringInt = { add: intAdd };
