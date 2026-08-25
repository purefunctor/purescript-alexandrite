import * as $foreign from "./foreign.js";

export function branched(choose) {
  return seed => {
    const $action = constructEffect("branch-action")(seed);
    const $effect = () => {
      const value = $action();
      if (choose) {
        return constructEffect("branch-then")(value)();
      } else {
        return constructEffect("branch-else")(value)();
      }
    };
    return $effect;
  };
}

export function patternLet(seed) {
  const $action = constructEffect("pattern-action")(seed);
  const $effect = () => {
    const value = $action();
    const $scrutinee = { selected: value };
    const selected = $scrutinee.selected;
    return constructEffect("pattern-result")(selected)();
  };
  return $effect;
}

export const constructEffect = $foreign["constructEffect"];
