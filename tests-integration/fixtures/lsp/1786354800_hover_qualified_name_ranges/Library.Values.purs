module Library.Values where

type Value = Int

value :: Value
value = 42

add :: Int -> Int -> Int
add left right = left + right

infixl 6 add as +
