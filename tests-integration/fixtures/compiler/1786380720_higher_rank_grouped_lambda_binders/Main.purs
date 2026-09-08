module Main where

data Unit = Unit

class Identity value

foreign import consume :: (Unit -> forall value. Identity value => value -> value) -> Unit

test :: Unit
test = consume \_ value -> value
