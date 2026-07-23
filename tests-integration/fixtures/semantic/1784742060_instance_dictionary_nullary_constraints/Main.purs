module Main where

import Prim.TypeError (class Fail, Text)

class Required :: Type -> Constraint
class Required value

class Example :: Type -> Constraint
class Example a where
  example :: Boolean

implementation ::
  Fail (Text "first deferred error") =>
  Required Int =>
  Fail (Text "second deferred error") =>
  Boolean
implementation = boolean

foreign import boolean :: Boolean

instance
  ( Fail (Text "first deferred error")
  , Required Int
  , Fail (Text "second deferred error")
  ) =>
  Example Int where
  example = implementation
