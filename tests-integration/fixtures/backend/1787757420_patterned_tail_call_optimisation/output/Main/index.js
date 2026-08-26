import * as $foreign from "./foreign.js";
export function patternedTail($record) {
  const value = $record.value;
  if (equalInt(value)(0 | 0)) {
    return value;
  } else {
    return patternedTail({ value: decrementInt(value) });
  }
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
