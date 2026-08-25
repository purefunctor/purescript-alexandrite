import * as $foreign from "./foreign.js";

export function identity(value) {
  return value;
}

export function mapped(value) {
  const $function = mark("map-function")(identity);
  const $action = constructEffect("map-action")(value);
  const $effect = () => {
    const $value = $action();
    return $function($value);
  };
  return $effect;
}

export function applied(value) {
  const $functionAction = constructEffect("apply-function-action")(identity);
  const $argumentAction = constructEffect("apply-value-action")(value);
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

export const constructEffect = $foreign["constructEffect"];
export const mark = $foreign["mark"];
