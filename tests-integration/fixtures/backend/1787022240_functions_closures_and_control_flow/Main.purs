module Main where

apply :: forall a b. (a -> b) -> a -> b
apply function value = function value

capture :: Int -> Int -> Int
capture captured = \_ -> captured

choose :: Boolean -> Int -> Int -> Int
choose condition left right = if condition then left else right

partial :: Int -> Int
partial = choose true 42

literalCase :: Int -> String
literalCase value = case value of
  0 -> "zero"
  _ -> "other"

higherOrder :: Int
higherOrder = apply (capture 42) 0
