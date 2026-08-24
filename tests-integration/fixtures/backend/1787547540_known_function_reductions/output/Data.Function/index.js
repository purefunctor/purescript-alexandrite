export function apply($function) {
  return argument => {
    return $function(argument);
  };
}

export function applyFlipped(argument) {
  return $function => {
    return $function(argument);
  };
}
