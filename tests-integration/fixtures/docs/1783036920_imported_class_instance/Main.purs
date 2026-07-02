module Main where

import Rendering (class Render)

-- | Local choice type.
data Choice = Choice

-- | Imported class instance for a local type.
instance renderChoice :: Render Choice where
  render _ = "choice"
