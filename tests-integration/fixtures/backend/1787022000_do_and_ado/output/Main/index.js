import * as $foreign from "./foreign.js";

export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];

export const sequential = (() => {
  return () => {
    const first = firstAction();
    return secondAction(first)();
  };
})();

export const independent = (() => {
  const $function = first => second => ({ first: first, second: second });
  return () => {
    let $function$1;
    $function$1 = $function(firstAction());
    return $function$1(independentAction());
  };
})();

export const pureValue = (() => {
  return () => {
    return 42 | 0;
  };
})();
