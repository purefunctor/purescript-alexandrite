module Main where

import Data.Show (class Show)

data Unshowable = Unshowable

newtype Identity a = Identity a

derive newtype instance Show (Identity Unshowable)
