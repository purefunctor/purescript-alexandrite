export function integer(argument0) {
  const matches = argument0 === (0 | 0);
  if (matches) {
    const literal = true;
    return literal;
  } else {
    const literal$1 = false;
    return literal$1;
  }
}

export function number(argument0) {
  const matches = argument0 === 1.5;
  if (matches) {
    const literal = true;
    return literal;
  } else {
    const literal$1 = false;
    return literal$1;
  }
}

export function character(argument0) {
  const matches = argument0 === "a";
  if (matches) {
    const literal = true;
    return literal;
  } else {
    const literal$1 = false;
    return literal$1;
  }
}

export function string(argument0) {
  const matches = argument0 === "alexandrite";
  if (matches) {
    const literal = true;
    return literal;
  } else {
    const literal$1 = false;
    return literal$1;
  }
}

export function boolean(argument0) {
  const matches = argument0 === true;
  if (matches) {
    const literal = true;
    return literal;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      const literal$1 = false;
      return literal$1;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}
