module Main where

data Unit = Unit

foreign import consume :: (Unit -> Partial => Int -> Int) -> Unit
foreign import use :: Unit -> Int -> Int

sectioned :: Unit
sectioned = consume (use _ _)

lowered :: Unit
lowered = consume (\unit value -> use unit value)
