module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit)
import Effect (Effect)

sharedBind :: Unit -> Effect { left :: Int, right :: Int }
sharedBind _ = bind (pure 42) (\value -> pure { left: value, right: value })
