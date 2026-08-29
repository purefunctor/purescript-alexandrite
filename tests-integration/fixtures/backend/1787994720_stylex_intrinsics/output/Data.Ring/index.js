import * as $foreign from "./foreign.js";
export function negate(dictionary) {
  return dictionary.negate;
}
export const intNegate = $foreign["intNegate"];
export const numberNegate = $foreign["numberNegate"];
export const ringInt = { negate: intNegate };
export const ringNumber = { negate: numberNegate };
