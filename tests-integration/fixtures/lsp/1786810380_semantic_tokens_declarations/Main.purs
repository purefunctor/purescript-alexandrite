module Main (Box(..), class Render, value, (<+>), type (:*:)) where

import Library (Source(..), class SourceClass, source, (<++>), type (:++:)) as Library

type role Box representational
data Box phantom = Box phantom

class Render a where
  render :: a -> String

instance renderBox :: Render (Box a) where
  render box = "box"

value :: Box Int
value = Box 42

append left right = left
infixr 5 append as <+>

type Product left right = Box left
infixr 6 type Product as :*:

-- semantic tokens
