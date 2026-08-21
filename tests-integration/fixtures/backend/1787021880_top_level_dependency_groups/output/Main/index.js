export function first(argument0) {
  if (argument0 === true) {
    return 1 | 0;
  } else {
    if (argument0 === false) {
      return second(true);
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function second(argument0) {
  if (argument0 === true) {
    return 2 | 0;
  } else {
    if (argument0 === false) {
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
