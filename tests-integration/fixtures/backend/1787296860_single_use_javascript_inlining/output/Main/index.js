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
  return (item => item)(value);
}

export function inlineCall($function) {
  return value => {
    return $function(value);
  };
}

export function inlineArray(value) {
  return [value];
}

export function inlineRecord(value) {
  return { value: value };
}

export function inlineCapturedClosure(captured) {
  return $boolean => captured;
}

export function inlineNestedClosureCapture(record) {
  return $boolean => (projected => $boolean => projected)(record.value);
}

export function keepMultiUseClosure(captured) {
  function keepMultiUseClosure$closure(captured) {
    return $boolean => {
      return captured;
    };
  }
  const closure = keepMultiUseClosure$closure(captured);
  return { first: closure, second: closure };
}

export function keepMultiUse(record) {
  const value = record.value;
  return { first: value, second: value };
}

export function inlineAcrossCall(record) {
  return $function => {
    return { projected: record.value, called: $function(true) };
  };
}

export function inlineOrderedCalls(first) {
  return second => {
    return { first: first(true), second: second(false) };
  };
}

export function keepReorderedCalls(first) {
  return second => {
    const call = first(true);
    return { first: second(false), second: call };
  };
}

export function keepMultiUseCall($function) {
  return value => {
    const call = $function(value);
    return { first: call, second: call };
  };
}

export function keepCallBeforeBranch(condition) {
  return $function => {
    return value => {
      const call = $function(value);
      if (condition) {
        return call;
      } else {
        return value;
      }
    };
  };
}

export function keepTestCall($function) {
  return value => {
    const call = $function(value);
    if (Array.isArray(call) && call.length === 0) {
      return true;
    } else {
      return false;
    }
  };
}
