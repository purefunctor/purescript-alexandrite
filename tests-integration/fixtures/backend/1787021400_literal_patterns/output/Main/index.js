export function integer($int) {
  if ($int === (0 | 0)) {
    return true;
  } else {
    return false;
  }
}

export function number($number) {
  if ($number === 1.5) {
    return true;
  } else {
    return false;
  }
}

export function character($char) {
  if ($char === "a") {
    return true;
  } else {
    return false;
  }
}

export function string($string) {
  if ($string === "alexandrite") {
    return true;
  } else {
    return false;
  }
}

export function boolean($boolean) {
  if ($boolean === true) {
    return true;
  } else {
    if ($boolean === false) {
      return false;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}
