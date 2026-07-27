module Main where

import Data.Eq (class Eq)
import Data.Ord (class Ord)

foreign import data Opaque :: Type

derive instance Eq Opaque
derive instance Ord Opaque
