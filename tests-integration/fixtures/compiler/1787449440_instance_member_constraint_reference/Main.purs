module Main where

import Data.Eq (class Eq, eq)
import Data.Function (on)

newtype Wrapper a = Wrapper a

unwrap :: forall a. Wrapper a -> a
unwrap (Wrapper value) = value

instance eqWrapper :: Eq a => Eq (Wrapper a) where
  eq = eq `on` unwrap
