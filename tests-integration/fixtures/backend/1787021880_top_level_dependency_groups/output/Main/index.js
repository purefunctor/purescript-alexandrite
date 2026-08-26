function $tail_first_second($state, $argument0) {
  while (true) {
    switch ($state) {
      // first
      case 0: {
        const $currentArgument0 = $argument0;
        if ($currentArgument0 === true) {
          return 1 | 0;
        }
        if ($currentArgument0 === false) {
          $argument0 = true;
          $state = 1;
          continue;
        }
        throw new Error("Pattern match failure");
      }
      // second
      case 1: {
        const $currentArgument0$1 = $argument0;
        if ($currentArgument0$1 === true) {
          return 2 | 0;
        }
        if ($currentArgument0$1 === false) {
          $argument0 = true;
          $state = 0;
          continue;
        }
        throw new Error("Pattern match failure");
      }
    }
  }
}
export function first($boolean) {
  return $tail_first_second(0, $boolean);
}
export function second($boolean$1) {
  return $tail_first_second(1, $boolean$1);
}
export const later = 42 | 0;
export const forward = later;
export const recursiveValue = /* @__PURE__ */ (() => {
  const $closure = (value) => {
    if (value) {
      return 3 | 0;
    } else {
      return recursivePeer.run(true);
    }
  };
  return { run: $closure };
})();
export const recursivePeer = /* @__PURE__ */ (() => {
  const $closure = (value) => {
    if (value) {
      return 4 | 0;
    } else {
      return recursiveValue.run(true);
    }
  };
  return { run: $closure };
})();
