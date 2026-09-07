import * as $foreign from "./foreign.js";
export function identity(value) {
  return value;
}
export function mapped(value) {
  const $function = mark("map-function")(identity);
  const $action = constructEffect("map-action")(value);
  return () => {
    return $function($action());
  };
}
export function applied(value) {
  const $functionAction = constructEffect("apply-function-action")(identity);
  const $argumentAction = constructEffect("apply-value-action")(value);
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
export const constructEffect = $foreign["constructEffect"];
export const mark = $foreign["mark"];
