export function mutual(condition) {
  const $tail_second_first = ($state, $argument0) => {
    while (true) {
      switch ($state) {
        // second
        case 0: {
          const $currentArgument0 = $argument0;
          if ($currentArgument0 === true) {
            return 2 | 0;
          }
          if ($currentArgument0 === false) {
            $argument0 = true;
            $state = 1;
            continue;
          }
          throw new Error("Pattern match failure");
        }
        // first
        case 1: {
          const $currentArgument0$1 = $argument0;
          if ($currentArgument0$1 === true) {
            return 1 | 0;
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
  };
  const second = ($boolean) => {
    return $tail_second_first(0, $boolean);
  };
  const first = ($boolean$1) => {
    return $tail_second_first(1, $boolean$1);
  };
  return first(condition);
}
export function capturedMutual(captured) {
  return (condition) => {
    const $tail_second_first = ($state, $argument0) => {
      while (true) {
        switch ($state) {
          // second
          case 0: {
            const $currentArgument0 = $argument0;
            if ($currentArgument0 === true) {
              return captured;
            }
            if ($currentArgument0 === false) {
              $argument0 = true;
              $state = 1;
              continue;
            }
            throw new Error("Pattern match failure");
          }
          // first
          case 1: {
            const $currentArgument0$1 = $argument0;
            if ($currentArgument0$1 === true) {
              return captured;
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
    };
    const second = ($boolean) => {
      return $tail_second_first(0, $boolean);
    };
    const first = ($boolean$1) => {
      return $tail_second_first(1, $boolean$1);
    };
    return first(condition);
  };
}
export function nestedRecursive(condition) {
  const go = ($boolean) => {
    if ($boolean === true) {
      const nested = go(false);
      return nested;
    }
    if ($boolean === false) {
      return 0 | 0;
    }
    throw new Error("Pattern match failure");
  };
  return go(condition);
}
