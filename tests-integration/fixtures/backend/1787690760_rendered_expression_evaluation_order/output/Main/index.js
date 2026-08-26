import * as $foreign from "./foreign.js";
export function ordered(branch) {
  return (shouldThrow) => {
    const $closure = (value) => {
      if (branch) {
        return observe("branch-true")(value);
      } else {
        return observe("branch-false")(value);
      }
    };
    return collect(observe("before")(observedRecord.value))($closure(failAt("middle")(shouldThrow)(2 | 0)))(observe("after")(3 | 0));
  };
}
export const collect = $foreign["collect"];
export const failAt = $foreign["failAt"];
export const observe = $foreign["observe"];
export const observedRecord = $foreign["observedRecord"];
export const reused = (() => {
  const value = observe("reused")(4 | 0);
  return [value, value];
})();
