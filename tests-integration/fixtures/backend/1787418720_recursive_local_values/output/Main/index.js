import * as $foreign from "./foreign.js";
import * as $runtime from "../runtime.js";

export function applicationRecursive(token) {
  function first$initialize(second) {
    function first$initialize$closure(second) {
      return condition => {
        if (condition) {
          return second()(false);
        } else {
          return 1 | 0;
        }
      };
    }
    return observe("first")(first$initialize$closure(second));
  }
  function second$initialize(first) {
    function second$initialize$closure(first) {
      return condition => {
        if (condition) {
          return first()(false);
        } else {
          return 2 | 0;
        }
      };
    }
    return observe("second")(second$initialize$closure(first));
  }
  let first$lazy;
  let second$lazy;
  first$lazy = $runtime.binding("first", () => first$initialize(second$lazy));
  second$lazy = $runtime.binding("second", () => second$initialize(first$lazy));
  const first = first$lazy();
  const second = second$lazy();
  return { result: first(true), trace: readTrace(token) };
}

export function caseRecursive(condition) {
  function go$initialize(condition, go) {
    if (condition === true) {
      function go$initialize$closure(go) {
        return current => {
          if (current) {
            return go()(false);
          } else {
            return 30 | 0;
          }
        };
      }
      return go$initialize$closure(go);
    } else {
      if (condition === false) {
        function go$initialize$closure$1($boolean) {
          return 31 | 0;
        }
        return go$initialize$closure$1;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  }
  let go$lazy;
  go$lazy = $runtime.binding("go", () => go$initialize(condition, go$lazy));
  return go$lazy()(true);
}

export function letRecursive(condition) {
  function go$initialize(go) {
    function go$initialize$1$closure(go) {
      return current => {
        if (current) {
          return go()(false);
        } else {
          return 40 | 0;
        }
      };
    }
    return go$initialize$1$closure(go);
  }
  let go$lazy;
  go$lazy = $runtime.binding("go", () => go$initialize(go$lazy));
  return go$lazy()(condition);
}

export function strictCycle($boolean) {
  function value$initialize(value) {
    return wrap(value());
  }
  let value$lazy;
  value$lazy = $runtime.binding("value", () => value$initialize(value$lazy));
  return value$lazy();
}

export function wrap(value) {
  return value;
}

export const same = $foreign["same"];
export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];

export const recordRecursive = (() => {
  function first$initialize(second) {
    return { value: second() };
  }
  function second$initialize(first) {
    function second$initialize$1$closure(first) {
      return condition => {
        if (condition) {
          return (first()).value(false);
        } else {
          return 20 | 0;
        }
      };
    }
    return second$initialize$1$closure(first);
  }
  let first$lazy;
  let second$lazy;
  first$lazy = $runtime.binding("first", () => first$initialize(second$lazy));
  second$lazy = $runtime.binding("second", () => second$initialize(first$lazy));
  const first = first$lazy();
  const second = second$lazy();
  return same(first.value)(second);
})();
