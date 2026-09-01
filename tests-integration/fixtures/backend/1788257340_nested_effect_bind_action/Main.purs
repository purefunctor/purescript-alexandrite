module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit(..))
import Effect (Effect)

nestedBind :: Unit -> Effect Int
nestedBind _ =
  bind
    (bind (pure Unit) (\_ -> pure 42))
    (\value -> pure value)
