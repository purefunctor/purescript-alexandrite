export function apply($function) {
  return argument => {
    return $function(argument);
  };
}

export function unsafeCoerce(value) {
  return value;
}

export const categoryFn = (() => {
  return { identity: value => value };
})();
