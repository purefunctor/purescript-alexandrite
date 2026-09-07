export function render(dictionary) {
  return dictionary.render;
}
export function renderWithIndent(dictionary) {
  return dictionary.renderWithIndent;
}
export function renderBoolean(failTextDict) {
  return {};
}
export function renderString(failTextDict) {
  return { render: (value) => value };
}
export const renderInt = {};
