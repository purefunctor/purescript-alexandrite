module Control.Category where

import Control.Semigroupoid (class Semigroupoid)

class Semigroupoid category <= Category (category :: Type -> Type -> Type) where
  identity :: forall value. category value value

instance categoryFn :: Category Function where
  identity value = value
