module Main where

data Unit = Unit

foreign import consume :: Int -> Unit

test :: Unit
test = consume (\value -> 0)
