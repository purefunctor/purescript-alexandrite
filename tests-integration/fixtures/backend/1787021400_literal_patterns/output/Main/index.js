export function integer(argument0) {
  if (argument0 === (0 | 0)) {
    return true;
  } else {
    return false;
  }
}

export function number(argument0) {
  if (argument0 === 1.5) {
    return true;
  } else {
    return false;
  }
}

export function character(argument0) {
  if (argument0 === "a") {
    return true;
  } else {
    return false;
  }
}

export function string(argument0) {
  if (argument0 === "alexandrite") {
    return true;
  } else {
    return false;
  }
}

export function boolean(argument0) {
  if (argument0 === true) {
    return true;
  } else {
    if (argument0 === false) {
      return false;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}
