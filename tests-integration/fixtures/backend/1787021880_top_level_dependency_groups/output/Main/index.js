export function first($boolean) {
  if ($boolean === true) {
    return 1 | 0;
  }
  if ($boolean === false) {
    return second(true);
  }
  throw new Error("Pattern match failure");
}

export function second($boolean) {
  if ($boolean === true) {
    return 2 | 0;
  }
  if ($boolean === false) {
    return first(true);
  }
  throw new Error("Pattern match failure");
}

export const later = 42 | 0;

export const forward = later;

export const recursiveValue = (() => {
  const $closure = value => {
    if (value) {
      return 3 | 0;
    } else {
      return recursivePeer.run(true);
    }
  };
  const $field = $closure;
  const $record = { run: $field };
  return $record;
})();

export const recursivePeer = (() => {
  const $closure = value => {
    if (value) {
      return 4 | 0;
    } else {
      return recursiveValue.run(true);
    }
  };
  const $field = $closure;
  const $record = { run: $field };
  return $record;
})();
