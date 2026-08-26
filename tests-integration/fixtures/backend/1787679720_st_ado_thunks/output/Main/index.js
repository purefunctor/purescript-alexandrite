import * as $foreign from "./foreign.js";

export function timedAdo(seed) {
  const $function = first => second => ({ first: first, second: second });
  const $action = constructST("ado-first")(seed);
  const $argumentAction = constructST("ado-second")({ seed: seed });
  return () => {
    let $function$1;
    $function$1 = $function($action());
    return $function$1($argumentAction());
  };
}

export function identity(value) {
  return value;
}

export function mapped(value) {
  const $function = mark("map-function")(identity);
  const $action = constructST("map-action")(value);
  return () => {
    return $function($action());
  };
}

export function applied(value) {
  const $functionAction = constructST("apply-function-action")(identity);
  const $argumentAction = constructST("apply-value-action")(value);
  return () => {
    return $functionAction()($argumentAction());
  };
}

export function capturedPure(value) {
  const $value = mark("pure-value")(value);
  return () => {
    return $value;
  };
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
