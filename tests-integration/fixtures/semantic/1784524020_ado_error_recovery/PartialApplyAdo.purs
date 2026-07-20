module PartialApplyAdo where

foreign import data Box :: Type -> Type

foreign import map :: forall a b. (a -> b) -> Box a -> Box b
foreign import apply :: forall a. a -> Box a
foreign import pure :: forall a. a -> Box a

foreign import boxedInt :: Box Int
