module PartialAdo where

foreign import data Box :: Type -> Type

foreign import map :: forall a. a -> Box a
foreign import apply :: forall a b. Box (a -> b) -> Box a -> Box b
foreign import pure :: Int

foreign import boxedInt :: Box Int
