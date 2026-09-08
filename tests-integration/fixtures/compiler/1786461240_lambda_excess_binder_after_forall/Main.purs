module Main where

data Unit = Unit

data Proxy :: Type -> Type
data Proxy value = Proxy

foreign import consume :: (Unit -> forall value. Proxy value) -> Unit

test :: Unit
test = consume \_ extra -> Proxy
