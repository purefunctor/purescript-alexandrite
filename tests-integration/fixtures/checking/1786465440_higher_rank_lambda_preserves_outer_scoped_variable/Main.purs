module Main where

foreign import value :: forall a. a

newtype Packed = Packed (forall r. (forall z. z -> r) -> r)

recover :: forall a. Packed -> a
recover (Packed run) = run (\_ -> value :: a)
