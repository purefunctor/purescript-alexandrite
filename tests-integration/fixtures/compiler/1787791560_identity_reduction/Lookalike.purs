module Lookalike where

class Identity (identity :: Type -> Type -> Type) where
  identity :: forall value. identity value value

instance categoryFn :: Identity Function where
  identity value = value
