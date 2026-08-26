import * as $foreign from "./foreign.js";
export function singleArgument(value) {
  if (equalInt(value)(0 | 0)) {
    return value;
  } else {
    return twoArguments(decrementInt(value))(0 | 0);
  }
}
export function twoArguments(value) {
  return (accumulator) => {
    if (equalInt(value)(0 | 0)) {
      return accumulator;
    } else {
      return singleArgument(decrementInt(value));
    }
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
