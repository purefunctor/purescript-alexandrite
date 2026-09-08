export function apply(nat) {
  return (fa) => {
    return nat(fa);
  };
}
