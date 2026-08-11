module Main where

data Unit = Unit

foreign import consume :: (Int -> Int) -> Unit

test :: Unit
test = consume \value extra -> value
