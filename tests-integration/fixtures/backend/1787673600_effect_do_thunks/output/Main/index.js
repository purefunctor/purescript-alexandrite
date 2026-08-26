import * as $foreign from "./foreign.js";
export function chained(seed) {
  const $action = constructEffect("first")(seed);
  return () => {
    const first = $action();
    const $action$1 = constructEffect("second")({ first });
    const second = $action$1();
    return constructEffect("third")({
      first,
      second
    })();
  };
}
export function discarded(seed) {
  const $action = constructEffect("discard-first")("Unit");
  return () => {
    const $unit = $action();
    const result = mark("discard-let")(seed);
    return constructEffect("discard-second")(result)();
  };
}
export function pureAfterBind(seed) {
  const $action = constructEffect("pure-action")(seed);
  return () => {
    const value = $action();
    const $value = mark("pure-body")({ value });
    return $value;
  };
}
export const constructEffect = $foreign["constructEffect"];
export const mark = $foreign["mark"];
