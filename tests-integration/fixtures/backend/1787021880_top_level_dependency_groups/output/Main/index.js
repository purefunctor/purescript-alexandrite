export function first($boolean) {
  if ($boolean === true) {
    return 1 | 0;
  } else {
    if ($boolean === false) {
      return second(true);
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function second($boolean) {
  if ($boolean === true) {
    return 2 | 0;
  } else {
    if ($boolean === false) {
      return first(true);
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export const later = 42 | 0;

export const forward = later;

export const recursiveValue = (() => {
  function recursiveValue$initialize$closure(value) {
    if (value) {
      return 3 | 0;
    } else {
      return recursivePeer.run(true);
    }
  }
  return { run: recursiveValue$initialize$closure };
})();

export const recursivePeer = (() => {
  function recursivePeer$initialize$closure(value) {
    if (value) {
      return 4 | 0;
    } else {
      return recursiveValue.run(true);
    }
  }
  return { run: recursivePeer$initialize$closure };
})();
