import * as $foreign from "./foreign.js";

export function ordered(branch) {
  return shouldThrow => {
    const $function = collect(observe("before")(observedRecord.value));
    const $closure = value => {
      if (branch) {
        return observe("branch-true")(value);
      } else {
        return observe("branch-false")(value);
      }
    };
    const $function$1 = $closure;
    const $argument = failAt("middle")(shouldThrow)(2 | 0);
    const $call = $function$1($argument);
    const $argument$1 = $call;
    const $call$1 = $function($argument$1);
    const $function$2 = $call$1;
    const $argument$2 = observe("after")(3 | 0);
    const $call$2 = $function$2($argument$2);
    return $call$2;
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
