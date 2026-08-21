import * as Data_Eq from "../Data.Eq/index.js";

function compareTwice$closure(dictionary0) {
  return left => {
    return right => {
      const call = dictionary0.eq(left);
      const call$1 = call(right);
      if (call$1) {
        const call$2 = dictionary0.eq(right);
        const call$3 = call$2(left);
        return call$3;
      } else {
        return false;
      }
    };
  };
}

function compareGenericArraysTwice$closure(dictionary1) {
  return left => {
    return right => {
      const call = Data_Eq.eqArray(dictionary1);
      const call$1 = call.eq(left);
      const call$2 = call$1(right);
      if (call$2) {
        const call$3 = call.eq(right);
        const call$4 = call$3(left);
        return call$4;
      } else {
        return false;
      }
    };
  };
}

function distinctGivens$closure(dictionary2, dictionary3) {
  return leftA => {
    return rightA => {
      return leftB => {
        return rightB => {
          function if$join$1(result$1) {
            return result$1;
          }

          const call = Data_Eq.eqArray(dictionary2);
          const call$1 = Data_Eq.eqArray(dictionary3);
          const call$2 = call.eq(leftA);
          const call$3 = call$2(rightA);
          if (call$3) {
            const call$4 = call.eq(rightA);
            const call$5 = call$4(leftA);
            return call$5;
          } else {
            const call$6 = call$1.eq(leftB);
            const call$7 = call$6(rightB);
            if (call$7) {
              const call$8 = call$1.eq(rightB);
              const call$9 = call$8(leftB);
              return if$join$1(call$9);
            } else {
              return if$join$1(false);
            }
          }
        };
      };
    };
  };
}

function compareSuperclassArraysTwice$closure(dictionary4) {
  return left => {
    return right => {
      const call = Data_Eq.eqArray(dictionary4.superclass62);
      const call$1 = call.eq(left);
      const call$2 = call$1(right);
      if (call$2) {
        const call$3 = call.eq(right);
        const call$4 = call$3(left);
        return call$4;
      } else {
        return false;
      }
    };
  };
}

function compareSuperclassTwice$closure(dictionary5) {
  return left => {
    return right => {
      const call = dictionary5.superclass62.eq(left);
      const call$1 = call(right);
      if (call$1) {
        const call$2 = dictionary5.superclass62.eq(right);
        const call$3 = call$2(left);
        return call$3;
      } else {
        return false;
      }
    };
  };
}

function lambdaScope$closure(lambdaLeft) {
  return lambdaRight => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(lambdaLeft);
    const call$2 = call$1(lambdaRight);
    if (call$2) {
      const call$3 = call.eq(lambdaRight);
      const call$4 = call$3(lambdaLeft);
      return call$4;
    } else {
      return false;
    }
  };
}

function whereIsolation$closure(helperLeft) {
  return helperRight => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(helperLeft);
    const call$2 = call$1(helperRight);
    return call$2;
  };
}

export function compareTwice(dictionary0) {
  const closure = compareTwice$closure(dictionary0);
  return closure;
}

export function compareIntsTwice(left) {
  return right => {
    const call = Data_Eq.eqInt.eq(left);
    const call$1 = call(right);
    if (call$1) {
      const call$2 = Data_Eq.eqInt.eq(right);
      const call$3 = call$2(left);
      return call$3;
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const call$3 = call.eq(right);
      const call$4 = call$3(left);
      return call$4;
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(left);
    const call$2 = call$1(right);
    return call$2;
  };
}

export function compareGenericArraysTwice(dictionary1) {
  const closure = compareGenericArraysTwice$closure(dictionary1);
  return closure;
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(call);
        const call$2 = call$1.eq(nestedLeft);
        const call$3 = call$2(nestedRight);
        if (call$3) {
          const call$4 = call$1.eq(nestedRight);
          const call$5 = call$4(nestedLeft);
          return call$5;
        } else {
          const call$6 = call.eq(left);
          const call$7 = call$6(right);
          return call$7;
        }
      };
    };
  };
}

export function distinctGivens(dictionary2) {
  return dictionary3 => {
    const closure = distinctGivens$closure(dictionary2, dictionary3);
    return closure;
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        function if$join$1(result$1) {
          return result$1;
        }

        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(Data_Eq.eqBoolean);
        const call$2 = call.eq(leftInt);
        const call$3 = call$2(rightInt);
        if (call$3) {
          const call$4 = call.eq(rightInt);
          const call$5 = call$4(leftInt);
          return call$5;
        } else {
          const call$6 = call$1.eq(leftBoolean);
          const call$7 = call$6(rightBoolean);
          if (call$7) {
            const call$8 = call$1.eq(rightBoolean);
            const call$9 = call$8(leftBoolean);
            return if$join$1(call$9);
          } else {
            return if$join$1(false);
          }
        }
      };
    };
  };
}

export function compareArraysThrice(left) {
  return right => {
    function if$join$1(result$1) {
      return result$1;
    }

    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const call$3 = call.eq(right);
      const call$4 = call$3(left);
      if (call$4) {
        const call$5 = call.eq(left);
        const call$6 = call$5(right);
        return if$join$1(call$6);
      } else {
        return if$join$1(false);
      }
    } else {
      return false;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const eqArray = Data_Eq.eqArray;
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = eqArray(call);
    const call$2 = call$1.eq(left);
    const call$3 = call$2(right);
    if (call$3) {
      const call$4 = call$1.eq(right);
      const call$5 = call$4(left);
      return call$5;
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(dictionary4) {
  const closure = compareSuperclassArraysTwice$closure(dictionary4);
  return closure;
}

export function compareSuperclassTwice(dictionary5) {
  const closure = compareSuperclassTwice$closure(dictionary5);
  return closure;
}

export function lambdaScope(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    const call$1 = call.eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const call$3 = lambdaScope$closure(left);
      const call$4 = call$3(right);
      return call$4;
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    const call = whereIsolation$closure(left);
    const call$1 = call(right);
    if (call$1) {
      const call$2 = Data_Eq.eqArray(Data_Eq.eqInt);
      const call$3 = call$2.eq(left);
      const call$4 = call$3(right);
      return call$4;
    } else {
      return false;
    }
  };
}

export function equationScope(argument0) {
  return argument1 => {
    return argument2 => {
      const call = Data_Eq.eqArray(Data_Eq.eqInt);
      const matches = argument0 === true;
      if (matches) {
        const call$1 = call.eq(argument1);
        const call$2 = call$1(argument2);
        return call$2;
      } else {
        const matches$1 = argument0 === false;
        if (matches$1) {
          const call$3 = call.eq(argument2);
          const call$4 = call$3(argument1);
          return call$4;
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = Data_Eq.eqInt.eq(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eqInt.eq(3 | 0)(4 | 0);
