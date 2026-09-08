export function termHole(argument) {
  const localInt = argument;
  const localString = "not relevant";
  throw new Error("Generated code reached a source error");
}
export function recordPunHole($record) {
  const punInt = $record.punInt;
  const punString = $record.punString;
  throw new Error("Generated code reached a source error");
}
export function instanceTypeHoleMember(dictionary) {
  return dictionary.instanceTypeHoleMember;
}
export const instanceTypeHole = { instanceTypeHoleMember: (value) => value };
