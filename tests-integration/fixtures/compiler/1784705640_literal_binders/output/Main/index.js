export function integer($int) {
  return ($a) => {
    if ($int === (-1 | 0)) {
      const value = $a;
      return value;
    }
    const value$1 = $a;
    return value$1;
  };
}
export function number($number) {
  return ($a) => {
    if ($number === -1.5) {
      const value = $a;
      return value;
    }
    const value$1 = $a;
    return value$1;
  };
}
export function string($string) {
  return ($a) => {
    if ($string === "life") {
      const value = $a;
      return value;
    }
    const value$1 = $a;
    return value$1;
  };
}
export function character($char) {
  return ($a) => {
    if ($char === "a") {
      const value = $a;
      return value;
    }
    const value$1 = $a;
    return value$1;
  };
}
export function boolean($boolean) {
  return ($a) => {
    if ($boolean === true) {
      const value = $a;
      return value;
    }
    if ($boolean === false) {
      const value$1 = $a;
      return value$1;
    }
    throw new Error("Pattern match failure");
  };
}
