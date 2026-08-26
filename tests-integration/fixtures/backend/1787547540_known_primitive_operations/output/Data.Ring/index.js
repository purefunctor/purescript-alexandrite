import * as $foreign from "./foreign.js";
export function sub(dictionary) {
  return dictionary.sub;
}
export function negate(dictionary) {
  return dictionary.negate;
}
export const intSubtract = $foreign["intSubtract"];
export const intNegate = $foreign["intNegate"];
export const ringInt = {
  sub: intSubtract,
  negate: intNegate
};
