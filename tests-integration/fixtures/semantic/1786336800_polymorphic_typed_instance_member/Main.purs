module Main where

foreign import unsafeCoerce :: forall a b. a -> b

class Convert :: (Type -> Type) -> (Type -> Type) -> Constraint
class Convert f g where
  convert :: forall a. f a -> g a
  convertExplicit :: forall a. f a -> g a

class Clash :: (Type -> Type) -> Constraint
class Clash f where
  clash :: forall a. f a -> f a

data F :: Type -> Type
data F a

data G :: Type -> Type
data G a

instance Convert F G where
  convert = (unsafeCoerce :: forall a. F a -> G a)

  convertExplicit :: forall a. F a -> G a
  convertExplicit = (unsafeCoerce :: forall a. F a -> G a)

instance Clash f where
  clash :: forall a. f a -> f a
  clash (value :: f a) =
    case value of
      (matched :: f a) -> local matched
    where
    local :: f a -> f a
    local = unsafeCoerce
