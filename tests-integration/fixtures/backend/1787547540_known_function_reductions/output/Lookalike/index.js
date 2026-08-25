export function apply($function) {
  return argument => {
    return $function(argument);
  };
}

export function identity(dictionary) {
  return dictionary.identity;
}

export function unsafeCoerce(value) {
  return value;
}

export const categoryFn = { identity: value => value };
