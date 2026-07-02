module Rendering where

-- | Render class.
class Render a where
  render :: a -> String
