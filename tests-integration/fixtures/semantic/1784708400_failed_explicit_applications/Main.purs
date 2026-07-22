module Main where

foreign import identityInt :: Int -> Int
foreign import integer :: Int

test :: Int
test = identityInt integer integer
