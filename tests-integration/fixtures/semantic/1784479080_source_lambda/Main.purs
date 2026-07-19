module Main where

test :: forall a. a -> a
test = \value -> value

test' = \value -> value
