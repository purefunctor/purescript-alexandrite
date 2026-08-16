module Main where

add left right = left

infixl 6 add as +++

negate value = value

record value = { value, nested: { current: value } }

access input = input.nested.current

update input value = input { value = value, nested { current = -value } }

conditional condition left right =
  if condition then left +++ right else left `add` right

operator = (+++)
