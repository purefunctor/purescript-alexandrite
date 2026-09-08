import * as $foreign from "./foreign.js";
export function timedAdo(seed) {
  const $function = (first) => (second) => ({
    first,
    second
  });
  const $action = constructEffect("ado-first")(seed);
  const $argumentAction = constructEffect("ado-second")({ seed });
  return () => {
    let $function$1;
    $function$1 = $function($action());
    return $function$1($argumentAction());
  };
}
export const constructEffect = $foreign["constructEffect"];
