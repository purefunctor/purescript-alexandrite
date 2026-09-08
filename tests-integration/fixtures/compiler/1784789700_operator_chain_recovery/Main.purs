module Main where

add :: Int -> Int -> Int
add left right = left

infixl 5 add as +

missingRightOperand = 1 +
