module Main where

class Render a where
  render :: a -> String
  renderWithIndent :: Int -> a -> String

instance Render Int
