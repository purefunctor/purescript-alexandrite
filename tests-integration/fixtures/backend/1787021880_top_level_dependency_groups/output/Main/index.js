function recursiveValue$initialize$closure(value) {
  if (value) {
    const literal = 3 | 0;
    return literal;
  } else {
    const recursivePeer$1 = recursivePeer;
    const run = recursivePeer$1.run;
    const literal$1 = true;
    const call = run(literal$1);
    return call;
  }
}

function recursivePeer$initialize$closure(value) {
  if (value) {
    const literal = 4 | 0;
    return literal;
  } else {
    const recursiveValue$1 = recursiveValue;
    const run = recursiveValue$1.run;
    const literal$1 = true;
    const call = run(literal$1);
    return call;
  }
}

export function first(argument0) {
  const matches = argument0 === true;
  if (matches) {
    const literal = 1 | 0;
    return literal;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      const second$1 = second;
      const literal$1 = true;
      const call = second$1(literal$1);
      return call;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function second(argument0) {
  const matches = argument0 === true;
  if (matches) {
    const literal = 2 | 0;
    return literal;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      const first$1 = first;
      const literal$1 = true;
      const call = first$1(literal$1);
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
