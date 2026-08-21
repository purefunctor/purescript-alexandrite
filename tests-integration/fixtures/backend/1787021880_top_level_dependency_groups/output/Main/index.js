function recursiveValue$initialize$closure(value) {
  if (value) {
    return 3 | 0;
  } else {
    const call = recursivePeer.run(true);
    return call;
  }
}

function recursivePeer$initialize$closure(value) {
  if (value) {
    return 4 | 0;
  } else {
    const call = recursiveValue.run(true);
    return call;
  }
}

export function first(argument0) {
  const matches = argument0 === true;
  if (matches) {
    return 1 | 0;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      const call = second(true);
      return call;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function second(argument0) {
  const matches = argument0 === true;
  if (matches) {
    return 2 | 0;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      const call = first(true);
      return call;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export const later = 42 | 0;

export const forward = later;

export const recursiveValue = { run: recursiveValue$initialize$closure };

export const recursivePeer = { run: recursivePeer$initialize$closure };
