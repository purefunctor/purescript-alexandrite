module Main where

import Prim.Ordering
import Prim.Int
import Type.Proxy

class Pick a b result | a b -> result

instance pickSame :: Compare a a EQ => Pick a a a
else instance pickFallback :: Pick a b b

choose :: forall a b result. Pick a b result => Proxy a -> Proxy b -> Proxy result
choose _ _ = Proxy

x = choose (Proxy @Int) (Proxy @Int)
