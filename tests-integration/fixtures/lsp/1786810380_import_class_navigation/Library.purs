module Library (class Render) where

class Render a where
  render :: a -> String
