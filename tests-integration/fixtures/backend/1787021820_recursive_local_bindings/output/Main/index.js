export function mutual(condition) {
  const second = $boolean => {
    if ($boolean === true) {
      return 2 | 0;
    }
    if ($boolean === false) {
      return first(true);
    }
    throw new Error("Pattern match failure");
  };
  const first = $boolean$1 => {
    if ($boolean$1 === true) {
      return 1 | 0;
    }
    if ($boolean$1 === false) {
      return second(true);
    }
    throw new Error("Pattern match failure");
  };
  return first(condition);
}

export function capturedMutual(captured) {
  return condition => {
    const second = $boolean => {
      if ($boolean === true) {
        return captured;
      }
      if ($boolean === false) {
        return first(true);
      }
      throw new Error("Pattern match failure");
    };
    const first = $boolean$1 => {
      if ($boolean$1 === true) {
        return captured;
      }
      if ($boolean$1 === false) {
        return second(true);
      }
      throw new Error("Pattern match failure");
    };
    return first(condition);
  };
}

export function nestedRecursive(condition) {
  const go = $boolean => {
    if ($boolean === true) {
      const nested = go(false);
      return nested;
    }
    if ($boolean === false) {
      return 0 | 0;
    }
    throw new Error("Pattern match failure");
  };
  return go(condition);
}
