module Main where

partialValue :: Partial => Int
partialValue = 0

test = if true then test else partialValue
