module Main where

identity :: forall @a. a -> a
identity value = value

test :: Int -> Int
test = identity @Int @String
