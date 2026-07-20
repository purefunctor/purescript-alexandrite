module ErrorAdo where

foreign import data Box :: Type -> Type

foreign import map :: Int
foreign import apply :: forall a b. Box (a -> b) -> Box a -> Box b
foreign import pure :: forall a. a -> Box a

foreign import boxedInt :: Box Int
