import * as $foreign from "./foreign.js";

export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];

export const sequential = (() => {
  const $action = firstAction;
  const $effect = () => {
    const first = $action();
    return secondAction(first)();
  };
  return $effect;
})();

export const independent = (() => {
  const $function = first => second => ({ first: first, second: second });
  const $action = firstAction;
  const $argumentAction = independentAction;
  const $effect = () => {
    let $function$1;
    const $value = $action();
    $function$1 = $function($value);
    const $argument = $argumentAction();
    return $function$1($argument);
  };
  return $effect;
})();

export const pureValue = (() => {
  const $value = 42 | 0;
  const $effect = () => {
    return $value;
  };
  return $effect;
})();
