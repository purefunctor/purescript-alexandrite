export function apply($function) {
  return (value) => {
    return $function(value);
  };
}
export const argument = apply((value) => value);
export const functionPosition = (($function) => $function)(apply);
