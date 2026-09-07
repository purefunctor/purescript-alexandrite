module Main where

import Data.Eq (class Eq, eq)
import Data.Show (class Show, show)

data Box = Box

derive instance Eq Box

equal :: Boolean
equal = eq Box Box

newtype Identity a = Identity a

derive newtype instance Show a => Show (Identity a)

rendered :: String
rendered = show (Identity 42)
