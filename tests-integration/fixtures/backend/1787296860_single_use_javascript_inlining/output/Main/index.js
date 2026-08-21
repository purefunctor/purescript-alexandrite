function inlineClosure$closure(item) {
  return item;
}

function keepCapturedClosure$closure(captured) {
  return argument0 => {
    return captured;
  };
}

export function identity(value) {
  return value;
}

export function inlineGlobal(value) {
  const call = identity(value);
  return call;
}

export function inlineProperty(record) {
  const call = identity(record.value);
  return call;
}

export function inlineLiteral(condition) {
  if (condition) {
    return true;
  } else {
    return false;
  }
}

export function inlineClosure(value) {
  const call = inlineClosure$closure(value);
  return call;
}

export function keepCall($function) {
  return value => {
    const call = $function(value);
    return call;
  };
}

export function keepArray(value) {
  const array = [value];
  return array;
}

export function keepRecord(value) {
  const record = { value: value };
  return record;
}

export function keepCapturedClosure(captured) {
  const closure = keepCapturedClosure$closure(captured);
  return closure;
}

export function keepMultiUse(record) {
  const value = record.value;
  const record$1 = { first: value, second: value };
  return record$1;
}

export function keepAcrossCall(record) {
  return $function => {
    const value = record.value;
    const call = $function(true);
    const record$1 = { projected: value, called: call };
    return record$1;
  };
}
