module Main where

data Unit = Unit

class Marker :: Type -> Constraint
class Marker value

foreign import consume :: (Unit -> Marker Int => Int) -> Unit

test :: Unit
test = consume \_ extra -> 0
