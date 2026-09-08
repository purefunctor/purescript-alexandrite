export function testGuarded($boolean) {
  if ($boolean === true) {
    if (false) {
      return 1 | 0;
    }
  }
  if ($boolean === false) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export function testGuardedBoth($boolean) {
  if ($boolean === true) {
    if (true) {
      return 1 | 0;
    }
  }
  if ($boolean === false) {
    if (true) {
      return 0 | 0;
    }
  }
  throw new Error("Pattern match failure");
}
