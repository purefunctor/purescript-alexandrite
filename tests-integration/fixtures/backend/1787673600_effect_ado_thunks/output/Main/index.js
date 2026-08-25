import * as $foreign from "./foreign.js";

export function timedAdo(seed) {
  const $function = first => second => ({ first: first, second: second });
  const $action = constructEffect("ado-first")(seed);
  const $argumentAction = constructEffect("ado-second")({ seed: seed });
  const $effect = () => {
    let $function$1;
    const $value = $action();
    $function$1 = $function($value);
    const $argument = $argumentAction();
    return $function$1($argument);
  };
  return $effect;
}

export const constructEffect = $foreign["constructEffect"];
