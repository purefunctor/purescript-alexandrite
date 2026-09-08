module Main where

import Prim.TypeError (class Fail, Text)

foreign import boolean :: Boolean

deferred :: Fail (Text "first deferred error") => Fail (Text "second deferred error") => Boolean
deferred = boolean
