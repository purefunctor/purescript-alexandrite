module Main where

data Pair a b = Pair a b

foreign import add :: Int -> Int -> Int

infixl 6 add as +

identity :: forall a. a -> a
identity value = value

visible :: forall @a. a -> a
visible value = value

typed = ((identity 1 :: Int))

visibleInt = visible @Int 1

visibleArray = visible @(Array Int) [1]

sum = 1 + 2 + 3

backtick = 1 `add` 2

section = (_ + 1) 2

choose condition whenTrue whenFalse =
  if condition then whenTrue else whenFalse

first pair = case pair of
  Pair value _ -> value

local value =
  let
    doubled = add value value
  in
    doubled

array = [1, 2, 3]

record = { value: 1, nested: { enabled: true } }

quoted = { "foo-bar": 1 }

quotedAccess input = input."foo-bar"

quotedUpdate input = input { "foo-bar" = 2 }

access input = input.nested.enabled

update input = input { value = 2, nested { enabled = false } }
