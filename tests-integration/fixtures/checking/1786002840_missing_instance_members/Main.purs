module Main where

import Prim.TypeError (class Fail, Text)

class Render a where
  render :: a -> String
  renderCompact :: a -> String
  renderVerbose :: a -> String

instance Render Int where
  render _ = "Int"
  renderCompact _ = "Int"
  render :: Int -> String
  renderVerbose :: Int -> String

data Unreachable

instance Fail (Text "unreachable") => Render Unreachable

data PartiallyUnreachable

instance Fail (Text "partially unreachable") => Render PartiallyUnreachable where
  render _ = "PartiallyUnreachable"
