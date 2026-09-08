module Main where

data Proxy :: forall kind. kind -> Type
data Proxy value = Proxy

class Grouped :: Type -> Type -> Constraint
class Grouped left right where
  grouped :: forall (value :: Type). Proxy value -> Proxy right -> Proxy value

instance Grouped (function left) (function right) where
  grouped :: forall (left :: Type). Proxy left -> Proxy (function right) -> Proxy left
  grouped (value :: Proxy left) (_ :: Proxy (function right)) =
    let
      local :: forall (left :: Type). Proxy left -> Proxy (function right) -> Proxy left
      local (localValue :: Proxy left) (_ :: Proxy (function right)) =
        localValue :: Proxy left
    in
      local value Proxy :: Proxy left
