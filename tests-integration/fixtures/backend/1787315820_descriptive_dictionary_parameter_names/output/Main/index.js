export const Wrapper = $value0 => ["Wrapper", $value0];

export function genericEqual(equalValueDict) {
  return left => right => equalValueDict.equal(left)(right);
}

export function arrayEqual(equalArrayValueDict) {
  return left => right => equalArrayValueDict.equal(left)(right);
}

export function wrapperEqual(equalWrapperValueDict) {
  return left => right => equalWrapperValueDict.equal(left)(right);
}

export function concreteEqual(equalIntDict) {
  return left => right => equalIntDict.equal(left)(right);
}

export function convertToInt(convertValueIntDict) {
  return value => convertValueIntDict.convert(value);
}

export function distinctEqual(equalLeftDict) {
  return equalRightDict => {
    return left1 => left2 => right1 => right2 => ({
      left: equalLeftDict.equal(left1)(left2),
      right: equalRightDict.equal(right1)(right2)
    });
  };
}

export function duplicateEqual(equalValueDict) {
  return equalValueDict$1 => {
    return left => right => equalValueDict$1.equal(left)(right);
  };
}

export function parameterCollision(equalValueDict) {
  function parameterCollision$closure(equalValueDict) {
    return equalValueDict$1 => {
      return left => {
        return right => {
          if (equalValueDict$1) {
            return equalValueDict.equal(left)(right);
          } else {
            return false;
          }
        };
      };
    };
  }
  return parameterCollision$closure(equalValueDict);
}

export function isAvailable(availableDict) {
  return availableDict.available;
}
