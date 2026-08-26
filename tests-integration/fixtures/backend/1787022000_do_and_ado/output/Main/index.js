import * as $foreign from "./foreign.js";
export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];
export const sequential = /* @__PURE__ */ (() => {
  return () => {
    const first = firstAction();
    return secondAction(first)();
  };
})();
export const independent = /* @__PURE__ */ (() => {
  const $function = (first) => (second) => ({
    first,
    second
  });
  return () => {
    let $function$1;
    $function$1 = $function(firstAction());
    return $function$1(independentAction());
  };
})();
export const pureValue = /* @__PURE__ */ (() => {
  return () => {
    return 42 | 0;
  };
})();
