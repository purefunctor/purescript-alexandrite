module Main where

import Control.Applicative (pure)
import Control.Apply (apply)
import Control.Bind (bind, discard)
import Data.Functor (map)
import Effect (Effect)

foreign import firstAction :: Effect Int
foreign import secondAction :: Int -> Effect String
foreign import independentAction :: Effect Boolean

sequential :: Effect String
sequential = do
  first <- firstAction
  secondAction first

independent :: Effect { first :: Int, second :: Boolean }
independent = ado
  first <- firstAction
  second <- independentAction
  in { first, second }
