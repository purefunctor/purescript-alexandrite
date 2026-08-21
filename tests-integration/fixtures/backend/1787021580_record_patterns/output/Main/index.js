export function select($record) {
  const first = $record.first;
  return $record.nested.second;
}
