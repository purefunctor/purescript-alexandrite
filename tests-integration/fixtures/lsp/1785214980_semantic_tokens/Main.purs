module Main (Tree(oops)) where

import Prim (Int)

data Tree a = Leaf a | Branch (Tree a) (Tree a)

class Render a where
  render :: a -> String

identity :: forall a. a -> a
identity value = value

example :: Tree String
example = Branch (Leaf "😀") (Leaf "right")

qualified :: Prim.Int
qualified = 42

record :: { field :: String }
record = { field: "value" }

multiline = """left
right"""

-- semantic tokens
