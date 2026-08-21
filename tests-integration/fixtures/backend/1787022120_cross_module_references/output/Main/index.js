import * as Library from "../Library/index.js";

export function unbox($box) {
  if (Array.isArray($box) && $box[0] === "Box") {
    return $box[1];
  } else {
    throw new Error("Pattern match failure");
  }
}

export const fromLibrary = unbox(Library.box);

export const directReference = Library.libraryValue;
