module Main where

data Unit = Unit

data Proxy :: Type -> Type
data Proxy value = Proxy

class Marker :: Type -> Constraint
class Marker value

foreign import consume ::
  (Unit -> forall value. Marker value => Proxy value) -> Unit

test :: Unit
test = consume \_ extra -> Proxy
