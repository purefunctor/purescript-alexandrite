module Main where

import Data.Generic.Rep (class Generic)

data Void
derive instance Generic Void _

data Unit = Unit
derive instance Generic Unit _

data Identity a = Identity a
derive instance Generic (Identity a) _

data Pair a b = Pair a b
derive instance Generic (Pair a b) _

data Either a b = Left a | Right b
derive instance Generic (Either a b) _

newtype Wrapper a = Wrapper a
derive instance Generic (Wrapper a) _
