import * as $foreign from "./foreign.js";
export function effectTail(value) {
  const $tail_effectTail = ($state, $argument0) => {
    while (true) {
      const $currentArgument0 = $argument0;
      if (equalInt($currentArgument0)(0 | 0)) {
        return () => {
          return [false, $currentArgument0];
        };
      } else {
        const $action = constructTick($currentArgument0);
        return () => {
          const $unit = $action();
          const $tailArgument = decrementInt($currentArgument0);
          return [
            true,
            0,
            $tailArgument
          ];
        };
      }
    }
  };
  const $initialStep = $tail_effectTail(0, value);
  return () => {
    let $step;
    $step = $initialStep;
    while (true) {
      const $result = $step();
      if (!$result[0]) {
        return $result[1];
      }
      $step = $tail_effectTail($result[1], $result[2]);
    }
  };
}
function $tail_effectMutualEven_effectMutualOdd($state, $argument0) {
  while (true) {
    switch ($state) {
      // effectMutualEven
      case 0: {
        const $currentArgument0 = $argument0;
        if (equalInt($currentArgument0)(0 | 0)) {
          return () => {
            return [false, true];
          };
        } else {
          const $action = constructTick($currentArgument0);
          return () => {
            const $unit = $action();
            const $tailArgument = decrementInt($currentArgument0);
            return [
              true,
              1,
              $tailArgument
            ];
          };
        }
      }
      // effectMutualOdd
      case 1: {
        const $currentArgument0$1 = $argument0;
        if (equalInt($currentArgument0$1)(0 | 0)) {
          return () => {
            return [false, false];
          };
        } else {
          const $action$1 = constructTick($currentArgument0$1);
          return () => {
            const $unit$1 = $action$1();
            const $tailArgument$1 = decrementInt($currentArgument0$1);
            return [
              true,
              0,
              $tailArgument$1
            ];
          };
        }
      }
    }
  }
}
export function effectMutualEven(value) {
  const $initialStep = $tail_effectMutualEven_effectMutualOdd(0, value);
  return () => {
    let $step;
    $step = $initialStep;
    while (true) {
      const $result = $step();
      if (!$result[0]) {
        return $result[1];
      }
      $step = $tail_effectMutualEven_effectMutualOdd($result[1], $result[2]);
    }
  };
}
export function effectMutualOdd(value$1) {
  const $initialStep$1 = $tail_effectMutualEven_effectMutualOdd(1, value$1);
  return () => {
    let $step$1;
    $step$1 = $initialStep$1;
    while (true) {
      const $result$1 = $step$1();
      if (!$result$1[0]) {
        return $result$1[1];
      }
      $step$1 = $tail_effectMutualEven_effectMutualOdd($result$1[1], $result$1[2]);
    }
  };
}
function $tail_effectMixedShort_effectMixedLong($state, $argument0, $argument1) {
  while (true) {
    switch ($state) {
      // effectMixedShort
      case 0: {
        const $currentArgument0 = $argument0;
        if (equalInt($currentArgument0)(0 | 0)) {
          return () => {
            return [false, $currentArgument0];
          };
        } else {
          const $action = constructTick($currentArgument0);
          return () => {
            const $unit = $action();
            const $tailArgument = decrementInt($currentArgument0);
            const $tailArgument$1 = 0 | 0;
            return [
              true,
              1,
              $tailArgument,
              $tailArgument$1
            ];
          };
        }
      }
      // effectMixedLong
      case 1: {
        const $currentArgument0$1 = $argument0;
        const $currentArgument1 = $argument1;
        if (equalInt($currentArgument0$1)(0 | 0)) {
          return () => {
            return [false, $currentArgument1];
          };
        } else {
          const $action$1 = constructTick($currentArgument0$1);
          return () => {
            const $unit$1 = $action$1();
            const $tailArgument$2 = decrementInt($currentArgument0$1);
            return [
              true,
              0,
              $tailArgument$2,
              null
            ];
          };
        }
      }
    }
  }
}
export function effectMixedShort(value) {
  const $initialStep = $tail_effectMixedShort_effectMixedLong(0, value, null);
  return () => {
    let $step;
    $step = $initialStep;
    while (true) {
      const $result = $step();
      if (!$result[0]) {
        return $result[1];
      }
      $step = $tail_effectMixedShort_effectMixedLong($result[1], $result[2], $result[3]);
    }
  };
}
export function effectMixedLong(value$1) {
  return (accumulator) => {
    const $initialStep$1 = $tail_effectMixedShort_effectMixedLong(1, value$1, accumulator);
    return () => {
      let $step$1;
      $step$1 = $initialStep$1;
      while (true) {
        const $result$1 = $step$1();
        if (!$result$1[0]) {
          return $result$1[1];
        }
        $step$1 = $tail_effectMixedShort_effectMixedLong($result$1[1], $result$1[2], $result$1[3]);
      }
    };
  };
}
export const equalInt = $foreign["equalInt"];
export const decrementInt = $foreign["decrementInt"];
export const constructTick = $foreign["constructTick"];
