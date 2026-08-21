export const Empty = ["Empty"];
export const One = $value0 => ["One", $value0];

export function booleanGuard(value) {
  if (value) {
    return 1 | 0;
  } else {
    if (true) {
      return 0 | 0;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function patternGuard(choice) {
  const matches = Array.isArray(choice) && choice[0] === "One";
  if (matches) {
    const value = choice[1];
    return value;
  } else {
    if (true) {
      return 0 | 0;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function caseBooleanGuard(value) {
  if (false) {
    const result$1 = 1 | 0;
    return result$1;
  } else {
    return 2 | 0;
  }
}

export function casePatternGuard(choice) {
  const matches = Array.isArray(choice) && choice[0] === "One";
  if (matches) {
    const value = choice[1];
    const result$1 = value;
    return result$1;
  } else {
    return 0 | 0;
  }
}

export function nestedCaseGuard(value) {
  function case$1() {
    return 3 | 0;
  }

  function case$join$1(result$2) {
    const result$1 = result$2;
    return result$1;
  }

  const matches = value === true;
  if (matches) {
    if (true) {
      if (false) {
        const result$3 = 1 | 0;
        return case$join$1(result$3);
      } else {
        return case$join$1(2 | 0);
      }
    } else {
      return case$1();
    }
  } else {
    return case$1();
  }
}
