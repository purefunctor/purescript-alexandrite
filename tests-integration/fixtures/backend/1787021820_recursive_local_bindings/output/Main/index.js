function second$function(first) {
  return argument0 => {
    if (argument0 === true) {
      return 2 | 0;
    } else {
      if (argument0 === false) {
        return first(true);
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function first$function(second) {
  return argument0 => {
    if (argument0 === true) {
      return 1 | 0;
    } else {
      if (argument0 === false) {
        return second(true);
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function second$function$1(captured, first) {
  return argument0 => {
    if (argument0 === true) {
      return captured;
    } else {
      if (argument0 === false) {
        return first(true);
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function first$function$1(captured, second) {
  return argument0 => {
    if (argument0 === true) {
      return captured;
    } else {
      if (argument0 === false) {
        return second(true);
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

export function mutual(condition) {
  const second = argument0 => second$function(first)(argument0);
  const first = argument0 => first$function(second)(argument0);
  return first(condition);
}

export function capturedMutual(captured) {
  return condition => {
    const second = argument0 => second$function$1(captured, first)(argument0);
    const first = argument0 => first$function$1(captured, second)(argument0);
    return first(condition);
  };
}
