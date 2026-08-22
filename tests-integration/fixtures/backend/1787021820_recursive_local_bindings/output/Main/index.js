export function mutual(condition) {
  function second$function(first) {
    return $boolean => {
      if ($boolean === true) {
        return 2 | 0;
      } else {
        if ($boolean === false) {
          return first(true);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  }
  function first$function(second) {
    return $boolean => {
      if ($boolean === true) {
        return 1 | 0;
      } else {
        if ($boolean === false) {
          return second(true);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  }
  const second = $boolean => second$function(first)($boolean);
  const first = $boolean => first$function(second)($boolean);
  return first(condition);
}

export function capturedMutual(captured) {
  return condition => {
    function second$function(captured, first) {
      return $boolean => {
        if ($boolean === true) {
          return captured;
        } else {
          if ($boolean === false) {
            return first(true);
          } else {
            throw new Error("Pattern match failure");
          }
        }
      };
    }
    function first$function(captured, second) {
      return $boolean => {
        if ($boolean === true) {
          return captured;
        } else {
          if ($boolean === false) {
            return second(true);
          } else {
            throw new Error("Pattern match failure");
          }
        }
      };
    }
    const second = $boolean => second$function(captured, first)($boolean);
    const first = $boolean => first$function(captured, second)($boolean);
    return first(condition);
  };
}

export function nestedRecursive(condition) {
  function go$function(go) {
    return $boolean => {
      if ($boolean === true) {
        return go(false);
      } else {
        if ($boolean === false) {
          return 0 | 0;
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  }
  const go = $boolean => go$function(go)($boolean);
  return go(condition);
}
