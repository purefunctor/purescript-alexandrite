function capture$closure(captured) {
  return argument0 => {
    return captured;
  };
}

export function apply($function) {
  return value => {
    const call = $function(value);
    return call;
  };
}

export function capture(captured) {
  const closure = capture$closure(captured);
  return closure;
}

export function choose(condition) {
  return left => {
    return right => {
      if (condition) {
        return left;
      } else {
        return right;
      }
    };
  };
}

export function literalCase(value) {
  const matches = value === (0 | 0);
  if (matches) {
    return "zero";
  } else {
    return "other";
  }
}

export const partial = choose(true)(42 | 0);

export const higherOrder = apply(capture(42 | 0))(0 | 0);
