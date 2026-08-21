export function add(left) {
  return right => {
    return left;
  };
}

export function identity(value) {
  return value;
}

export function increment(section60) {
  const call = add(section60);
  const call$1 = call(1 | 0);
  return call$1;
}

export function accessValue(section79) {
  return section79.value;
}

export const operatorApplication = add(1 | 0)(2 | 0);

export const visibleTypeApplication = identity(42 | 0);
