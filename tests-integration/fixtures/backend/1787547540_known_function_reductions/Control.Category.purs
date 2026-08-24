module Control.Category where

class Category (category :: Type -> Type -> Type) where
  identity :: forall value. category value value

instance categoryFn :: Category Function where
  identity value = value
