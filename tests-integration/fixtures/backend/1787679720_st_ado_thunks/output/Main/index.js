import * as $foreign from "./foreign.js";

export function timedAdo(seed) {
  const $function = first => second => ({ first: first, second: second });
  const $action = constructST("ado-first")(seed);
  const $argumentAction = constructST("ado-second")({ seed: seed });
  const $effect = () => {
    let $function$1;
    const $value = $action();
    $function$1 = $function($value);
    const $argument = $argumentAction();
    return $function$1($argument);
  };
  return $effect;
}

export function identity(value) {
  return value;
}

export function mapped(value) {
  const $function = mark("map-function")(identity);
  const $action = constructST("map-action")(value);
  const $effect = () => {
    const $value = $action();
    return $function($value);
  };
  return $effect;
}

export function applied(value) {
  const $functionAction = constructST("apply-function-action")(identity);
  const $argumentAction = constructST("apply-value-action")(value);
  const $effect = () => {
    const $function = $functionAction();
    const $argument = $argumentAction();
    return $function($argument);
  };
  return $effect;
}

export function capturedPure(value) {
  const $value = mark("pure-value")(value);
  const $effect = () => {
    return $value;
  };
  return $effect;
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
