module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit, unit)
import Effect (Effect)

nestedBind :: Unit -> Effect Int
nestedBind _ =
  bind
    (bind (pure unit) (\_ -> pure 42))
    (\value -> pure value)
