module Main where

foreign import unsafeCoerce :: forall a b. a -> b

class Convert f g where
  convert :: forall a. f a -> g a

data F a
data G a

instance Convert F G where
  convert = (unsafeCoerce :: forall a. F a -> G a)
