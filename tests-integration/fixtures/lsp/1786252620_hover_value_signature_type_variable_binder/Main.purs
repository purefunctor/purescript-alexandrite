module Main where

apply :: forall (constructor :: Type -> Type). constructor Int -> constructor Int
--               $
apply value = value
