export const Wrapper = $value0 => ["Wrapper", $value0];

export function equal(dictionary) {
  return dictionary.equal;
}

export function convert(dictionary) {
  return dictionary.convert;
}

export function available(dictionary) {
  return dictionary.available;
}

export function genericEqual(equalValueDict) {
  return left => right => equal(equalValueDict)(left)(right);
}

export function arrayEqual(equalArrayValueDict) {
  return left => right => equal(equalArrayValueDict)(left)(right);
}

export function wrapperEqual(equalWrapperValueDict) {
  return left => right => equal(equalWrapperValueDict)(left)(right);
}

export function concreteEqual(equalIntDict) {
  return left => right => equal(equalIntDict)(left)(right);
}

export function convertToInt(convertValueIntDict) {
  return value => convert(convertValueIntDict)(value);
}

export function distinctEqual(equalLeftDict) {
  return equalRightDict => {
    return left1 => left2 => right1 => right2 => ({
      left: equal(equalLeftDict)(left1)(left2),
      right: equal(equalRightDict)(right1)(right2)
    });
  };
}

export function duplicateEqual(equalValueDict) {
  return equalValueDict$1 => {
    return left => right => equal(equalValueDict$1)(left)(right);
  };
}

export function parameterCollision(equalValueDict) {
  function parameterCollision$closure(equalValueDict) {
    return equalValueDict$1 => {
      return left => {
        return right => {
          if (equalValueDict$1) {
            return equal(equalValueDict)(left)(right);
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
  return available(availableDict);
}
