export function integer($int) {
  if ($int === (0 | 0)) {
    return true;
  }
  return false;
}
export function number($number) {
  if ($number === 1.5) {
    return true;
  }
  return false;
}
export function character($char) {
  if ($char === "a") {
    return true;
  }
  return false;
}
export function matchesEscapedDoubleQuote($char) {
  if ($char === "\"") {
    return true;
  }
  return false;
}
export function string($string) {
  if ($string === "alexandrite") {
    return true;
  }
  return false;
}
export function boolean($boolean) {
  if ($boolean === true) {
    return true;
  }
  if ($boolean === false) {
    return false;
  }
  throw new Error("Pattern match failure");
}
export const escapedDoubleQuote = "\"";
