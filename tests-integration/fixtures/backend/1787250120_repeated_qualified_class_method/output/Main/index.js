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
    if (eqIntDictEq(left)(right)) {
      return eqIntDictEq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    if (eqArrayIntDictEq(left)(right)) {
      return eqArrayIntDictEq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return eqArrayIntDictEq(left)(right);
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
        if (eqArrayArrayIntDictEq(nestedLeft)(nestedRight)) {
          return eqArrayArrayIntDictEq(nestedRight)(nestedLeft);
        } else {
          return eqArrayIntDictEq(left)(right);
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
        if (eqArrayIntDictEq(leftInt)(rightInt)) {
          return eqArrayIntDictEq(rightInt)(leftInt);
        } else {
          if (eqArrayBooleanDictEq(leftBoolean)(rightBoolean)) {
            return eqArrayBooleanDictEq(rightBoolean)(leftBoolean);
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
    if (eqArrayIntDictEq(left)(right)) {
      if (eqArrayIntDictEq(right)(left)) {
        return eqArrayIntDictEq(left)(right);
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
    if (eqArrayArrayIntDictEq(left)(right)) {
      return eqArrayArrayIntDictEq(right)(left);
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
    if (eqArrayIntDictEq(left)(right)) {
      const $closure = lambdaLeft => {
        return lambdaRight => {
          if (eqArrayIntDictEq(lambdaLeft)(lambdaRight)) {
            return eqArrayIntDictEq(lambdaRight)(lambdaLeft);
          } else {
            return false;
          }
        };
      };
      return $closure(left)(right);
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    if ((helperLeft => helperRight => eqArrayIntDictEq(helperLeft)(helperRight))(left)(right)) {
      return eqArrayIntDictEq(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope($boolean) {
  return $array => {
    return $array$1 => {
      if ($boolean === true) {
        const left = $array;
        const right = $array$1;
        return eqArrayIntDictEq(left)(right);
      }
      if ($boolean === false) {
        const left$1 = $array;
        const right$1 = $array$1;
        return eqArrayIntDictEq(right$1)(left$1);
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

const eqIntDictEq = Data_Eq.eq(Data_Eq.eqInt);

export const firstComparison = eqIntDictEq(1 | 0)(2 | 0);

export const secondComparison = eqIntDictEq(3 | 0)(4 | 0);

export const eqRecursive = $lazy_eqRecursive();

const eqArrayIntDict = Data_Eq.eqArray(Data_Eq.eqInt);
const eqArrayBooleanDict = Data_Eq.eqArray(Data_Eq.eqBoolean);
const eqArrayIntDictEq = Data_Eq.eq(eqArrayIntDict);
const eqArrayArrayIntDict = Data_Eq.eqArray(eqArrayIntDict);
const eqArrayBooleanDictEq = Data_Eq.eq(eqArrayBooleanDict);
const eqArrayArrayIntDictEq = Data_Eq.eq(eqArrayArrayIntDict);
