import * as Library from "../Library/index.js";
export function unbox($box) {
  if ($box[0] === "Box") {
    const [, value] = $box;
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
}
export const fromLibrary = unbox(Library.box);
export const directReference = Library.libraryValue;
