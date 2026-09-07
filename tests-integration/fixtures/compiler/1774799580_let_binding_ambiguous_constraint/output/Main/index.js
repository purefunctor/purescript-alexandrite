import * as $foreign from "./foreign.js";
export const delay = $foreign["delay"];
export const test = /* @__PURE__ */ (() => {
  const $action = delay(42 | 0);
  const $effect = () => {
    const $unit = $action();
    return delay(42 | 0)();
  };
  const implementation = $effect;
  return implementation;
})();
