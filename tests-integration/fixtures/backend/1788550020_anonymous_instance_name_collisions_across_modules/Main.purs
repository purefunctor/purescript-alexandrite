module Main where

import Library (Proxy(..))
import Library as Library

class Pick :: Symbol -> Constraint
class Pick symbol where
  pick :: Proxy symbol -> Int

instance Pick "first" where
  pick _ = 10

result :: { third :: Int, first :: Int, second :: Int, named :: Int, repeated :: Int, local :: Int }
result =
  { third: Library.pick (Proxy :: Proxy "third")
  , first: Library.pick (Proxy :: Proxy "first")
  , second: Library.pick (Proxy :: Proxy "second")
  , named: Library.pick (Proxy :: Proxy "named")
  , repeated: Library.pick (Proxy :: Proxy "third")
  , local: pick (Proxy :: Proxy "first")
  }
