module Main where

import Prim.TypeError (class Fail, Text)

class Render a where
  render :: a -> String
  renderWithIndent :: Int -> a -> String

instance Render Int

instance Fail (Text "empty") => Render Boolean

instance Fail (Text "partial") => Render String where
  render value = value
