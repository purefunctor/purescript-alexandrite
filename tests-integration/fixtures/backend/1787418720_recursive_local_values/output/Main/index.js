import * as $foreign from "./foreign.js";
import * as $runtime from "../runtime.js";

export function applicationRecursive(token) {
  let $lazy_first;
  let $lazy_second;
  $lazy_first = $runtime.binding("first", () => {
    const $function = observe("first");
    const $closure = condition => {
      if (condition) {
        return $lazy_second()(false);
      } else {
        return 1 | 0;
      }
    };
    const $argument = $closure;
    const $call = $function($argument);
    return $call;
  });
  $lazy_second = $runtime.binding("second", () => {
    const $function$1 = observe("second");
    const $closure$1 = condition$1 => {
      if (condition$1) {
        return $lazy_first()(false);
      } else {
        return 2 | 0;
      }
    };
    const $argument$1 = $closure$1;
    const $call$1 = $function$1($argument$1);
    return $call$1;
  });
  const first = $lazy_first();
  const second = $lazy_second();
  return { result: first(true), trace: readTrace(token) };
}

export function caseRecursive(condition) {
  let $lazy_go;
  $lazy_go = $runtime.binding("go", () => {
    if (condition === true) {
      const $closure = current => {
        if (current) {
          return $lazy_go()(false);
        } else {
          return 30 | 0;
        }
      };
      return $closure;
    }
    if (condition === false) {
      return $boolean => 31 | 0;
    }
    throw new Error("Pattern match failure");
  });
  const go = $lazy_go();
  return go(true);
}

export function letRecursive(condition) {
  let $lazy_go;
  $lazy_go = $runtime.binding("go", () => {
    const wrapped = current => {
      if (current) {
        return $lazy_go()(false);
      } else {
        return 40 | 0;
      }
    };
    return wrapped;
  });
  const go = $lazy_go();
  return go(condition);
}

export function strictCycle($boolean) {
  let $lazy_value;
  $lazy_value = $runtime.binding("value", () => {
    return wrap($lazy_value());
  });
  const value = $lazy_value();
  return value;
}

export function wrap(value) {
  return value;
}

export const same = $foreign["same"];
export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];

export const recordRecursive = (() => {
  let $lazy_first;
  let $lazy_second;
  $lazy_first = $runtime.binding("first", () => {
    return { value: $lazy_second() };
  });
  $lazy_second = $runtime.binding("second", () => {
    const $closure = condition => {
      if (condition) {
        return ($lazy_first()).value(false);
      } else {
        return 20 | 0;
      }
    };
    return $closure;
  });
  const first = $lazy_first();
  const second = $lazy_second();
  return same(first.value)(second);
})();
