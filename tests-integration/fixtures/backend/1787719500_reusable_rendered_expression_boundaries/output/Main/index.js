import * as $foreign from "./foreign.js";

export const Box = $value0 => ["Box", $value0];

export function stableApplication($function) {
  return condition => {
    const $function$1 = $function;
    let $result;
    if (condition) {
      $result = observe("application-then")(1 | 0);
    } else {
      $result = observe("application-else")(2 | 0);
    }
    return $function$1($result);
  };
}

export function observedApplication(condition) {
  const $function = observed.apply;
  let $result;
  if (condition) {
    $result = observe("observed-then")(3 | 0);
  } else {
    $result = observe("observed-else")(4 | 0);
  }
  return $function($result);
}

export function stableArray(value) {
  return condition => {
    const $element = value;
    let $result;
    if (condition) {
      $result = observe("array-then")(5 | 0);
    } else {
      $result = observe("array-else")(6 | 0);
    }
    return [$element, $result];
  };
}

export function observedArray(condition) {
  const $element = observed.value;
  let $result;
  if (condition) {
    $result = observe("observed-array-then")(7 | 0);
  } else {
    $result = observe("observed-array-else")(8 | 0);
  }
  return [$element, $result];
}

export function stablePure(value) {
  const $value = value;
  const $effect = () => {
    return $value;
  };
  return $effect;
}

export function stableMap($function) {
  const $function$1 = $function;
  const $action = constructEffect("stable-map")(9 | 0);
  const $effect = () => {
    const $value = $action();
    return $function$1($value);
  };
  return $effect;
}

export function observedMap($boolean) {
  const $function = observed.apply;
  const $action = constructEffect("observed-map")(10 | 0);
  const $effect = () => {
    const $value = $action();
    return $function($value);
  };
  return $effect;
}

export function mixedApply($boolean) {
  const $functionAction = constructEffect("mixed-function")(value => observe("mixed-call")(value));
  const $action = constructEffect("mixed-argument-first")(18 | 0);
  const $effect = () => {
    const $function = $functionAction();
    let $argument;
    const value$1 = $action();
    $argument = constructEffect("mixed-argument-second")(value$1)();
    return $function($argument);
  };
  return $effect;
}

export function joinedEffect(condition) {
  let $result;
  if (condition) {
    const $function = observed.apply;
    const $action = constructEffect("joined-then")(16 | 0);
    const $effect = () => {
      const $value = $action();
      return $function($value);
    };
    $result = $effect;
  } else {
    const $function$1 = observed.apply;
    const $action$1 = constructEffect("joined-else")(17 | 0);
    const $effect$1 = () => {
      const $value$1 = $action$1();
      return $function$1($value$1);
    };
    $result = $effect$1;
  }
  return [$result];
}

export function joinedPattern(condition) {
  let $result;
  if (condition) {
    $result = Box(observe("pattern-then")(11 | 0));
  } else {
    $result = Box(observe("pattern-else")(12 | 0));
  }
  const $scrutinee = $result;
  if (Array.isArray($scrutinee) && $scrutinee[0] === "Box") {
    const value = $scrutinee[1];
    return value;
  }
  throw new Error("Pattern match failure");
}

export const constructEffect = $foreign["constructEffect"];
export const observe = $foreign["observe"];
export const observed = $foreign["observed"];
