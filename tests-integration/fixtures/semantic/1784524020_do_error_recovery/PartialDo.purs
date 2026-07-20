module PartialDo where

foreign import data Box :: Type -> Type

foreign import bind :: forall a. Box a -> Box a
foreign import discard :: forall a b. Box a -> (a -> Box b) -> Box b
foreign import pure :: forall a. a -> Box a

foreign import boxedInt :: Box Int
