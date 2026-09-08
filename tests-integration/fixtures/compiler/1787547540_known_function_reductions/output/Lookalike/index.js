export function apply($function) {
  return (argument) => {
    return $function(argument);
  };
}
