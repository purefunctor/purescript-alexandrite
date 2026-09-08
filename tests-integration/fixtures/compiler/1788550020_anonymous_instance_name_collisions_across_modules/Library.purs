module Library where

data Proxy :: Symbol -> Type
data Proxy symbol = Proxy

class Pick :: Symbol -> Constraint
class Pick symbol where
  pick :: Proxy symbol -> Int

pick1 :: Int
pick1 = 90

instance Pick "first" where
  pick _ = 1

instance Pick "second" where
  pick _ = 2

instance pick4 :: Pick "named" where
  pick _ = 4

instance Pick "third" where
  pick _ = 3
