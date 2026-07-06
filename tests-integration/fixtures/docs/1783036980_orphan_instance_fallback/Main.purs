module Main where

import Remote (Remote)
import Rendering (class Render)

-- | Imported class instance for an imported type.
instance renderRemote :: Render Remote where
  render _ = "remote"
