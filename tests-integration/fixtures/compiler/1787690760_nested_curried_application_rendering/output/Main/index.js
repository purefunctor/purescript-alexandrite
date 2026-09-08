import * as $foreign from "./foreign.js";
export function render(state) {
  const $closure = (value) => {
    if (value) {
      return "active";
    } else {
      return "inactive";
    }
  };
  return node("main")([attribute("root")])([node("span")([attribute($closure(state))])([text("first")]), node("span")([])([text("second")])]);
}
export const node = $foreign["node"];
export const attribute = $foreign["attribute"];
export const text = $foreign["text"];
