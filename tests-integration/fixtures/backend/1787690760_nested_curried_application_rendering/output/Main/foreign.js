export const node = name => attributes => children =>
  `<${name} ${attributes.join(" ")}>${children.join("")}</${name}>`;

export const attribute = value => `class=${value}`;

export const text = value => value;
