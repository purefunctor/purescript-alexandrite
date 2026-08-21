function recursiveValue$initialize$closure(value) {
  if (value) {
    return 3 | 0;
  } else {
    return (0, recursivePeer.run)(true);
  }
}

function recursivePeer$initialize$closure(value) {
  if (value) {
    return 4 | 0;
  } else {
    return (0, recursiveValue.run)(true);
  }
}

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

export const recursiveValue = { run: recursiveValue$initialize$closure };

export const recursivePeer = { run: recursivePeer$initialize$closure };
