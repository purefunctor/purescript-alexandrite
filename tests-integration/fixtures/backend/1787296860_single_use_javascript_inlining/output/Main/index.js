export function identity(value) {
  return value;
}
export function inlineGlobal(value) {
  return identity(value);
}
export function inlineProperty(record) {
  return identity(record.value);
}
export function inlineLiteral(condition) {
  if (condition) {
    return true;
  } else {
    return false;
  }
}
export function inlineClosure(value) {
  return ((item) => item)(value);
}
export function inlineCall($function) {
  return (value) => {
    return $function(value);
  };
}
export function inlineArray(value) {
  return [value];
}
export function inlineRecord(value) {
  return { value };
}
export function inlineCapturedClosure(captured) {
  return ($boolean) => captured;
}
export function inlineAlias(value) {
  return value;
}
export function inlineRepeatedAlias(value) {
  return {
    first: value,
    second: value
  };
}
export function inlineSingleUseProperty(record) {
  return identity(record.value);
}
export function keepMultiUseClosure(captured) {
  const closure = ($boolean) => {
    return captured;
  };
  return {
    first: closure,
    second: closure
  };
}
export function keepMultiUse(record) {
  const value = record.value;
  return {
    first: value,
    second: value
  };
}
export function inlineAcrossCall(record) {
  return ($function) => {
    return {
      projected: record.value,
      called: $function(true)
    };
  };
}
export function inlineOrderedCalls(first) {
  return (second) => {
    return {
      first: first(true),
      second: second(false)
    };
  };
}
export function keepReorderedCalls(first) {
  return (second) => {
    const firstResult = first(true);
    const secondResult = second(false);
    return {
      first: secondResult,
      second: firstResult
    };
  };
}
export function keepMultiUseCall($function) {
  return (value) => {
    const result = $function(value);
    return {
      first: result,
      second: result
    };
  };
}
export function keepCallBeforeBranch(condition) {
  return ($function) => {
    return (value) => {
      const result = $function(value);
      if (condition) {
        return result;
      } else {
        return value;
      }
    };
  };
}
export function keepTestCall($function) {
  return (value) => {
    const $scrutinee = $function(value);
    if (Array.isArray($scrutinee) && $scrutinee.length === 0) {
      return true;
    }
    return false;
  };
}
