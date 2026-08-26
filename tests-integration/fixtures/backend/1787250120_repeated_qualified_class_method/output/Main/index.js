import * as Data_Eq from "../Data.Eq/index.js";
import * as $runtime from "../runtime.js";

export function compareTwice(eqADict) {
  const $closure = left => {
    return right => {
      if (Data_Eq.eq(eqADict)(left)(right)) {
        return Data_Eq.eq(eqADict)(right)(left);
      } else {
        return false;
      }
    };
  };
  return $closure;
}

export function compareIntsTwice(left) {
  return right => {
    if (Data_Eq.eq(Data_Eq.eqInt)(left)(right)) {
      return Data_Eq.eq(Data_Eq.eqInt)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
    if (Data_Eq.eq(eqArrayDict)(left)(right)) {
      return Data_Eq.eq(eqArrayDict)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
  };
}

export function compareGenericArraysTwice(eqADict) {
  const $closure = left => {
    return right => {
      const eqArrayDict = Data_Eq.eqArray(eqADict);
      if (Data_Eq.eq(eqArrayDict)(left)(right)) {
        return Data_Eq.eq(eqArrayDict)(right)(left);
      } else {
        return false;
      }
    };
  };
  return $closure;
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
        const eqArrayDict$1 = Data_Eq.eqArray(eqArrayDict);
        if (Data_Eq.eq(eqArrayDict$1)(nestedLeft)(nestedRight)) {
          return Data_Eq.eq(eqArrayDict$1)(nestedRight)(nestedLeft);
        } else {
          return Data_Eq.eq(eqArrayDict)(left)(right);
        }
      };
    };
  };
}

export function distinctGivens(eqADict) {
  return eqBDict => {
    const $closure = leftA => {
      return rightA => {
        return leftB => {
          return rightB => {
            const eqArrayDict = Data_Eq.eqArray(eqADict);
            const eqArrayDict$1 = Data_Eq.eqArray(eqBDict);
            if (Data_Eq.eq(eqArrayDict)(leftA)(rightA)) {
              return Data_Eq.eq(eqArrayDict)(rightA)(leftA);
            } else {
              if (Data_Eq.eq(eqArrayDict$1)(leftB)(rightB)) {
                return Data_Eq.eq(eqArrayDict$1)(rightB)(leftB);
              } else {
                return false;
              }
            }
          };
        };
      };
    };
    return $closure;
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
        const eqArrayDict$1 = Data_Eq.eqArray(Data_Eq.eqBoolean);
        if (Data_Eq.eq(eqArrayDict)(leftInt)(rightInt)) {
          return Data_Eq.eq(eqArrayDict)(rightInt)(leftInt);
        } else {
          if (Data_Eq.eq(eqArrayDict$1)(leftBoolean)(rightBoolean)) {
            return Data_Eq.eq(eqArrayDict$1)(rightBoolean)(leftBoolean);
          } else {
            return false;
          }
        }
      };
    };
  };
}

export function compareArraysThrice(left) {
  return right => {
    const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
    if (Data_Eq.eq(eqArrayDict)(left)(right)) {
      if (Data_Eq.eq(eqArrayDict)(right)(left)) {
        return Data_Eq.eq(eqArrayDict)(left)(right);
      } else {
        return false;
      }
    } else {
      return false;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqArray(Data_Eq.eqInt));
    if (Data_Eq.eq(eqArrayDict)(left)(right)) {
      return Data_Eq.eq(eqArrayDict)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(orderedADict) {
  const $closure = left => {
    return right => {
      const eqArrayDict = Data_Eq.eqArray(orderedADict.Eq0());
      if (Data_Eq.eq(eqArrayDict)(left)(right)) {
        return Data_Eq.eq(eqArrayDict)(right)(left);
      } else {
        return false;
      }
    };
  };
  return $closure;
}

export function compareSuperclassTwice(orderedADict) {
  const $closure = left => {
    return right => {
      const Eq0Dict = orderedADict.Eq0();
      if (Data_Eq.eq(Eq0Dict)(left)(right)) {
        return Data_Eq.eq(Eq0Dict)(right)(left);
      } else {
        return false;
      }
    };
  };
  return $closure;
}

export function lambdaScope(left) {
  return right => {
    if (Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right)) {
      const $closure = lambdaLeft => {
        return lambdaRight => {
          const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
          if (Data_Eq.eq(eqArrayDict)(lambdaLeft)(lambdaRight)) {
            return Data_Eq.eq(eqArrayDict)(lambdaRight)(lambdaLeft);
          } else {
            return false;
          }
        };
      };
      const $function = $closure;
      const $argument = left;
      const $call = $function($argument);
      const $function$1 = $call;
      const $argument$1 = right;
      const $call$1 = $function$1($argument$1);
      return $call$1;
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    if ((helperLeft => helperRight => Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(helperLeft)(
      helperRight
    ))(left)(right)) {
      return Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope($boolean) {
  return $array => {
    return $array$1 => {
      const eqArrayDict = Data_Eq.eqArray(Data_Eq.eqInt);
      if ($boolean === true) {
        const left = $array;
        const right = $array$1;
        return Data_Eq.eq(eqArrayDict)(left)(right);
      }
      if ($boolean === false) {
        const left$1 = $array;
        const right$1 = $array$1;
        return Data_Eq.eq(eqArrayDict)(right$1)(left$1);
      }
      throw new Error("Pattern match failure");
    };
  };
}

export function compareRecursiveTwice(left) {
  return right => {
    if (Data_Eq.eq($lazy_eqRecursive())(left)(right)) {
      return Data_Eq.eq($lazy_eqRecursive())(right)(left);
    } else {
      return false;
    }
  };
}

const $lazy_eqRecursive = $runtime.binding("eqRecursive", () => {
  return { eq: left => right => Data_Eq.eq($lazy_eqRecursive())(left)(right) };
});

export const firstComparison = Data_Eq.eq(Data_Eq.eqInt)(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eq(Data_Eq.eqInt)(3 | 0)(4 | 0);

export const eqRecursive = $lazy_eqRecursive();
