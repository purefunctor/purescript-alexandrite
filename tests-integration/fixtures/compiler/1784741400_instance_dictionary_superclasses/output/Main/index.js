import * as $foreign from "./foreign.js";
export function compare(dictionary) {
  return dictionary.compare;
}
export function ordInt(eqIntDict) {
  return {
    Eq0: () => eqIntDict,
    compare: compareImpl
  };
}
export const compareImpl = $foreign["compareImpl"];
