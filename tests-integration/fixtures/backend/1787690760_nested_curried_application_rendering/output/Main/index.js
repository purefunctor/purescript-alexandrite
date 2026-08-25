import * as $foreign from "./foreign.js";

export function render(state) {
  const $function = node("main")([attribute("root")]);
  const $function$1 = node("span");
  const $function$2 = attribute;
  const $closure = value => {
    if (value) {
      return "active";
    } else {
      return "inactive";
    }
  };
  const $function$3 = $closure;
  const $argument = state;
  const $call = $function$3($argument);
  const $argument$1 = $call;
  const $call$1 = $function$2($argument$1);
  const $element = $call$1;
  const $array = [$element];
  const $argument$2 = $array;
  const $call$2 = $function$1($argument$2);
  const $function$4 = $call$2;
  const $argument$3 = [text("first")];
  const $call$3 = $function$4($argument$3);
  const $element$1 = $call$3;
  const $element$2 = node("span")([])([text("second")]);
  const $array$1 = [$element$1, $element$2];
  const $argument$4 = $array$1;
  const $call$4 = $function($argument$4);
  return $call$4;
}

export const node = $foreign["node"];
export const attribute = $foreign["attribute"];
export const text = $foreign["text"];
