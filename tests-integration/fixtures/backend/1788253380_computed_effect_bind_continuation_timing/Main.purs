module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit(..))
import Effect (Effect)

foreign import makeContinuation :: Unit -> (Unit -> Effect Int)

computedBind :: Unit -> Effect Int
computedBind _ = bind (pure Unit) (makeContinuation Unit)
