export function integer(argument0) {
  const matches = argument0 === (0 | 0);
  if (matches) {
    return true;
  } else {
    return false;
  }
}

export function number(argument0) {
  const matches = argument0 === 1.5;
  if (matches) {
    return true;
  } else {
    return false;
  }
}

export function character(argument0) {
  const matches = argument0 === "a";
  if (matches) {
    return true;
  } else {
    return false;
  }
}

export function string(argument0) {
  const matches = argument0 === "alexandrite";
  if (matches) {
    return true;
  } else {
    return false;
  }
}

export function boolean(argument0) {
  const matches = argument0 === true;
  if (matches) {
    return true;
  } else {
    const matches$1 = argument0 === false;
    if (matches$1) {
      return false;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}
