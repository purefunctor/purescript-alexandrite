module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Data.Unit (Unit(..))
import Effect (Effect)

foreign import makeContinuation :: Unit -> (Unit -> Effect Int)

localBind :: Unit -> Effect Int
localBind _ =
  let continuation = makeContinuation Unit
  in bind (pure Unit) continuation
