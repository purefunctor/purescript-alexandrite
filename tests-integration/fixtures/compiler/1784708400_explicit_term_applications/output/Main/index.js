export function identity(value) {
  return value;
}
export function applyIdentity(value) {
  return identity(identity(value));
}
