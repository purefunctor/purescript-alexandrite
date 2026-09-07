export function select($record) {
  const first = $record.first;
  const second = $record.nested.second;
  return second;
}
