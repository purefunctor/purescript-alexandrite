module Main where

data Proxy :: forall k. k -> Type
data Proxy a = Proxy

class ShowP :: forall k. k -> Constraint
class ShowP a where
  showP :: Proxy a -> String

instance showPFirst :: ShowP a where
  showP _ = "first"

instance showPSecond :: ShowP a where
  showP _ = "second"

value :: String
value = showP (Proxy :: Proxy Int)
