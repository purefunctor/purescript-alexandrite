module Main where

foreign import unsafeCoerce :: forall a b. a -> b

class Convert :: (Type -> Type) -> (Type -> Type) -> Constraint
class Convert f g where
  convert :: forall a. f a -> g a

data F :: Type -> Type
data F a

data G :: Type -> Type
data G a

instance Convert F G where
  convert = (unsafeCoerce :: forall a. F a -> G a)
